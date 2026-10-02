use crate::manager::{self, Harness, LinkWarning, ScanResponse};
use crate::settings::{self, Settings};
use serde::Deserialize;
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, MutexGuard, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::ipc::Channel;

const STAGING_PREFIX: &str = ".ai-skill-manager-staging-";
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
static GENERATION: AtomicU64 = AtomicU64::new(1);
static RUNNING: AtomicBool = AtomicBool::new(false);
static EXECUTION: OnceLock<Mutex<()>> = OnceLock::new();
static SCANS: OnceLock<Mutex<HashMap<String, ScanBaseline>>> = OnceLock::new();
static PLANS: OnceLock<Mutex<HashMap<String, ManagePlan>>> = OnceLock::new();

#[derive(Clone)]
struct SkillBaseline {
    folder_name: String,
    source: PathBuf,
    fingerprint: String,
    links: Vec<LinkWarning>,
    warnings: Vec<String>,
}

#[derive(Clone)]
struct ScanBaseline {
    harness: Harness,
    source_parent: Option<PathBuf>,
    destination_resolved: PathBuf,
    warnings: Vec<String>,
    skills: HashMap<String, SkillBaseline>,
    installed: HashMap<String, InstalledBaseline>,
}

#[derive(Clone)]
struct InstalledBaseline {
    folder_name: String,
    path: PathBuf,
    fingerprint: String,
    links: Vec<LinkWarning>,
    warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationAction {
    Install,
    Update,
    Uninstall,
}

#[derive(Clone)]
struct ManageItem {
    folder_name: String,
    source: Option<SkillBaseline>,
    installed: Option<InstalledBaseline>,
}

#[derive(Clone)]
struct ManagePlan {
    action: OperationAction,
    harness: Harness,
    destination_resolved: PathBuf,
    source_parent: Option<PathBuf>,
    items: Vec<ManageItem>,
    warnings: Vec<LinkWarning>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedSkill {
    pub folder_name: String,
    pub link_warnings: Vec<LinkWarning>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareResponse {
    pub token: String,
    pub action: OperationAction,
    pub eligible: Vec<PreparedSkill>,
    pub skipped: Vec<String>,
    pub destination_path: PathBuf,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub kind: String,
    pub folder_name: Option<String>,
    pub completed: usize,
    pub total: usize,
    pub success: Option<bool>,
    pub message: Option<String>,
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::Acquire)
}

pub fn invalidate_plans() {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    if let Some(plans) = PLANS.get() {
        plans.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
    if let Some(scans) = SCANS.get() {
        scans.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

pub fn acquire() -> Result<MutexGuard<'static, ()>, String> {
    EXECUTION
        .get_or_init(|| Mutex::new(()))
        .try_lock()
        .map_err(|_| {
            "Another operation is already running. Please try again when it finishes.".into()
        })
}

pub fn begin_scan() -> u64 {
    invalidate_plans();
    GENERATION.load(Ordering::Acquire)
}

pub fn record_scan(scan: &ScanResponse, generation: u64) {
    if generation != GENERATION.load(Ordering::Acquire) {
        return;
    }
    scans().lock().unwrap_or_else(|e| e.into_inner()).clear();
    let skills = scan
        .skills
        .iter()
        .filter_map(|row| {
            if !matches!(row.status.as_str(), "missing" | "different") {
                return None;
            }
            Some((
                identity_key(&row.folder_name),
                SkillBaseline {
                    folder_name: row.folder_name.clone(),
                    source: scan.resolved_source_path.as_ref()?.join(&row.folder_name),
                    fingerprint: row.source_fingerprint.clone()?,
                    links: row.source_link_warnings.clone(),
                    warnings: row.warnings.clone(),
                },
            ))
        })
        .collect();
    let installed = scan
        .skills
        .iter()
        .filter_map(|row| {
            let mut warnings = row.warnings.clone();
            if let Some(error) = &row.error {
                warnings.push(error.clone());
            }
            Some((
                identity_key(&row.folder_name),
                InstalledBaseline {
                    folder_name: row.folder_name.clone(),
                    path: row.destination_path.clone()?,
                    fingerprint: row.destination_fingerprint.clone()?,
                    links: row.destination_link_warnings.clone(),
                    warnings,
                },
            ))
        })
        .collect();
    scans().lock().unwrap_or_else(|e| e.into_inner()).insert(
        scan.revision.clone(),
        ScanBaseline {
            harness: scan.harness.clone(),
            source_parent: scan.resolved_source_path.clone(),
            destination_resolved: scan.resolved_destination_path.clone(),
            warnings: scan.warnings.clone(),
            skills,
            installed,
        },
    );
}

pub fn prepare_operation(
    settings_now: &Settings,
    harness: Harness,
    revision: &str,
    action: OperationAction,
    selected: &[String],
) -> Result<PrepareResponse, String> {
    let baseline = scans()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(revision)
        .cloned()
        .ok_or_else(|| {
            "This comparison is stale. Refresh and select the skills again.".to_string()
        })?;
    if baseline.harness != harness {
        return Err("This comparison belongs to another harness. Refresh and try again.".into());
    }
    let destination = settings::configured_destination(settings_now, &harness)
        .map(Path::to_path_buf)
        .ok_or_else(|| "No destination is configured for this harness.".to_string())?;
    let destination_resolved = manager::resolved_path(&destination)?;
    if manager::path_key_for_ops(&destination_resolved)
        != manager::path_key_for_ops(&baseline.destination_resolved)
    {
        return Err(
            "Destination settings changed after this scan. Refresh and confirm again.".into(),
        );
    }
    let needs_source = !matches!(action, OperationAction::Uninstall);
    let source_parent = if needs_source {
        let source = baseline.source_parent.clone().ok_or_else(|| {
            "Choose an accessible source folder before this operation.".to_string()
        })?;
        if !resolves_to(settings_now.source.as_deref(), &source)
            || manager::paths_overlap(&source, &destination_resolved)
        {
            return Err("Source or destination changed or overlaps after this scan. Refresh and confirm again.".into());
        }
        Some(source)
    } else {
        None
    };
    let mut eligible = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in selected {
        let key = identity_key(name);
        if !seen.insert(key.clone()) {
            continue;
        }
        let installed = baseline.installed.get(&key).cloned();
        if matches!(action, OperationAction::Uninstall) && installed.is_none() {
            skipped.push(format!("{name} (not eligible in this scan)"));
            continue;
        }
        if matches!(action, OperationAction::Update) && installed.is_none() {
            skipped.push(format!("{name} (not installed)"));
            continue;
        }
        if matches!(action, OperationAction::Install) && installed.is_some() {
            skipped.push(format!("{name} (already installed)"));
            continue;
        }
        let source = if needs_source {
            let Some(source) = baseline.skills.get(&key).cloned() else {
                skipped.push(format!("{name} (source is invalid or unavailable)"));
                continue;
            };
            if manager::fingerprint(&source.source).ok().as_deref() != Some(&source.fingerprint) {
                return Err(format!(
                    "{} changed after the comparison. Refresh and confirm again.",
                    source.folder_name
                ));
            }
            Some(source)
        } else {
            None
        };
        if let Some(installed) = &installed {
            if manager::operation_fingerprint(&installed.path)
                .ok()
                .as_deref()
                != Some(&installed.fingerprint)
            {
                return Err(format!(
                    "{} changed after the comparison. Refresh and confirm again.",
                    installed.folder_name
                ));
            }
        }
        if let Some(source) = &source {
            if manager::paths_overlap(&source.source, &destination_resolved) {
                return Err(format!(
                    "Source skill {} overlaps the destination.",
                    source.folder_name
                ));
            }
            for warning in &source.links {
                if manager::paths_overlap(Path::new(&warning.target), &destination_resolved) {
                    return Err(format!(
                        "Link in {} targets the destination tree.",
                        source.folder_name
                    ));
                }
            }
        }
        if matches!(action, OperationAction::Uninstall) {
            ensure_uninstall_safe_from_source(
                settings_now,
                &item_path(&destination_resolved, installed.as_ref().unwrap()),
            )?;
        }
        eligible.push(ManageItem {
            folder_name: installed.as_ref().map_or_else(
                || source.as_ref().unwrap().folder_name.clone(),
                |installed| installed.folder_name.clone(),
            ),
            source,
            installed,
        });
    }
    if eligible.is_empty() {
        return Err(format!(
            "None of the selected skills is eligible to {}.",
            match action {
                OperationAction::Install => "install",
                OperationAction::Update => "update",
                OperationAction::Uninstall => "uninstall",
            }
        ));
    }
    let warnings = eligible
        .iter()
        .flat_map(|item| {
            item.source
                .iter()
                .flat_map(|source| source.links.clone())
                .chain(
                    item.installed
                        .iter()
                        .flat_map(|installed| installed.links.clone()),
                )
        })
        .collect();
    let token = new_token();
    plans().lock().unwrap_or_else(|e| e.into_inner()).clear();
    plans().lock().unwrap_or_else(|e| e.into_inner()).insert(
        token.clone(),
        ManagePlan {
            action,
            harness,
            destination_resolved,
            source_parent,
            items: eligible.clone(),
            warnings,
        },
    );
    Ok(PrepareResponse {
        token,
        action,
        eligible: eligible
            .iter()
            .map(|item| PreparedSkill {
                folder_name: item.folder_name.clone(),
                link_warnings: item
                    .source
                    .iter()
                    .flat_map(|source| source.links.clone())
                    .chain(
                        item.installed
                            .iter()
                            .flat_map(|installed| installed.links.clone()),
                    )
                    .collect(),
                warnings: item
                    .source
                    .iter()
                    .flat_map(|source| source.warnings.clone())
                    .chain(
                        item.installed
                            .iter()
                            .flat_map(|installed| installed.warnings.clone()),
                    )
                    .collect(),
            })
            .collect(),
        skipped,
        destination_path: destination,
        warnings: baseline.warnings,
    })
}

pub fn execute_locked(
    token: &str,
    settings_now: &Settings,
    acknowledge_links: bool,
    channel: Channel<OperationEvent>,
) -> Result<(), String> {
    RUNNING.store(true, Ordering::Release);
    let result = execute_operation(
        token,
        settings_now,
        acknowledge_links,
        recycle_to_system,
        |event| {
            let _ = channel.send(event);
        },
    );
    RUNNING.store(false, Ordering::Release);
    result
}

fn execute_operation(
    token: &str,
    settings_now: &Settings,
    acknowledge_links: bool,
    recycle: impl FnMut(&Path) -> Result<(), String>,
    emit: impl FnMut(OperationEvent),
) -> Result<(), String> {
    execute_operation_with_place(
        token,
        settings_now,
        acknowledge_links,
        recycle,
        |from, to| fs::rename(from, to).map_err(|error| error.to_string()),
        emit,
    )
}

fn execute_operation_with_place(
    token: &str,
    settings_now: &Settings,
    acknowledge_links: bool,
    mut recycle: impl FnMut(&Path) -> Result<(), String>,
    place: impl Fn(&Path, &Path) -> Result<(), String>,
    mut emit: impl FnMut(OperationEvent),
) -> Result<(), String> {
    let plan = plans()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(token)
        .ok_or_else(|| {
            "This operation confirmation expired. Refresh and confirm again.".to_string()
        })?;
    if !plan.warnings.is_empty() && !acknowledge_links {
        return Err("Acknowledge the listed links and junctions before continuing.".into());
    }
    ensure_plan_current(settings_now, &plan)?;
    if matches!(plan.action, OperationAction::Install) {
        fs::create_dir_all(&plan.destination_resolved).map_err(|error| {
            format!(
                "Could not create destination {}: {error}",
                plan.destination_resolved.display()
            )
        })?;
        ensure_plan_current(settings_now, &plan)?;
    }
    let stage = if !matches!(plan.action, OperationAction::Uninstall) {
        let stage = plan.destination_resolved.join(new_stage_name());
        fs::create_dir(&stage)
            .map_err(|error| format!("Could not create staging folder: {error}"))?;
        Some(stage)
    } else {
        None
    };
    let total = plan.items.len();
    let mut completed = 0;
    let mut preserved = Vec::new();
    for item in &plan.items {
        emit(OperationEvent {
            kind: "progress".into(),
            folder_name: Some(item.folder_name.clone()),
            completed,
            total,
            success: None,
            message: Some(format!("{} {}", action_verb(plan.action), item.folder_name)),
        });
        let result = match (&plan.action, &stage) {
            (OperationAction::Uninstall, _) => {
                uninstall_one(item, &plan, settings_now, &mut recycle)
            }
            (OperationAction::Update, Some(stage)) => {
                update_one(item, &plan, settings_now, stage, &mut recycle, &place)
            }
            (OperationAction::Install, Some(stage)) => {
                install_manage_one(item, &plan, settings_now, stage, &place)
            }
            _ => unreachable!(),
        };
        completed += 1;
        if let Err((error, preserve)) = result {
            if let Some(path) = preserve {
                preserved.push(path.clone());
            }
            emit(OperationEvent {
                kind: "result".into(),
                folder_name: Some(item.folder_name.clone()),
                completed,
                total,
                success: Some(false),
                message: Some(error),
            });
        } else {
            emit(OperationEvent {
                kind: "result".into(),
                folder_name: Some(item.folder_name.clone()),
                completed,
                total,
                success: Some(true),
                message: Some(format!("{} successfully.", action_past(plan.action))),
            });
        }
    }
    let cleanup = stage.as_ref().and_then(|stage| {
        if preserved.is_empty() { cleanup_stage(stage, &plan.destination_resolved).err() }
        else { Some(format!("Prepared replacement retained at {}; original folder is in the system trash. Recover it manually if needed.", preserved.iter().map(|path| path.display().to_string()).collect::<Vec<_>>().join(", "))) }
    });
    emit(OperationEvent {
        kind: "finished".into(),
        folder_name: None,
        completed,
        total,
        success: Some(cleanup.is_none()),
        message: Some(
            cleanup.unwrap_or_else(|| format!("{} batch finished.", action_verb(plan.action))),
        ),
    });
    Ok(())
}

fn uninstall_one(
    item: &ManageItem,
    plan: &ManagePlan,
    settings_now: &Settings,
    recycle: &mut impl FnMut(&Path) -> Result<(), String>,
) -> Result<(), (String, Option<PathBuf>)> {
    let installed = item
        .installed
        .as_ref()
        .expect("uninstall destination validated");
    let path = installed.path.as_path();
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    ensure_uninstall_safe_from_source(
        settings_now,
        &item_path(&plan.destination_resolved, installed),
    )
    .map_err(|error| (error, None))?;
    verify_installed(installed, path)?;
    manager::validate_recyclable(path).map_err(|error| (error, None))?;
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    verify_installed(installed, path)?;
    recycle(path).map_err(|error| (format!("Could not move the installed folder to the system trash; it was left in place: {error}"), None))
}

fn update_one(
    item: &ManageItem,
    plan: &ManagePlan,
    settings_now: &Settings,
    stage: &Path,
    recycle: &mut impl FnMut(&Path) -> Result<(), String>,
    place: &impl Fn(&Path, &Path) -> Result<(), String>,
) -> Result<(), (String, Option<PathBuf>)> {
    let source = item.source.as_ref().expect("update source validated");
    let installed = item
        .installed
        .as_ref()
        .expect("update destination validated");
    let destination = &installed.path;
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    if manager::fingerprint(&source.source).ok().as_deref() != Some(&source.fingerprint) {
        return Err((
            "Source files changed after confirmation. Refresh and confirm again.".into(),
            None,
        ));
    }
    verify_installed(installed, destination)?;
    let staged = stage.join(&item.folder_name);
    fs::create_dir(&staged)
        .map_err(|error| (format!("Could not prepare staging folder: {error}"), None))?;
    manager::copy_materialized(
        &source.source,
        &staged,
        &[plan.destination_resolved.clone(), stage.to_path_buf()],
    )
    .map_err(|error| (error, None))?;
    if !staged_matches(&source.source, &staged).map_err(|error| (error, None))? {
        return Err((
            "Staged content did not match the source; the installed folder was left intact.".into(),
            None,
        ));
    }
    if manager::fingerprint(&source.source).ok().as_deref() != Some(&source.fingerprint) {
        return Err((
            "Source files changed while staging. Refresh and confirm again.".into(),
            None,
        ));
    }
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    verify_installed(installed, destination)?;
    manager::validate_recyclable(destination).map_err(|error| (error, None))?;
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    verify_installed(installed, destination)?;
    if manager::fingerprint(&source.source).ok().as_deref() != Some(&source.fingerprint) {
        return Err((
            "Source files changed before recycling. Refresh and confirm again.".into(),
            None,
        ));
    }
    recycle(destination).map_err(|error| {
        (
            format!(
                "Could not move the old folder to the system trash; it was left in place: {error}"
            ),
            None,
        )
    })?;
    ensure_plan_current(settings_now, plan).map_err(|error| (format!("The old folder was recycled from {} but destination settings changed. Prepared copy retained at {}. Restore the original manually from the system trash. {error}", destination.display(), staged.display()), Some(staged.clone())))?;
    place(&staged, destination).map_err(|error| (format!("The old folder was recycled from {}, but the replacement could not be placed. Prepared copy retained at {}. Restore the original manually from the system trash. Placement error: {error}", destination.display(), staged.display()), Some(staged)))
}

fn verify_installed(
    item: &InstalledBaseline,
    path: &Path,
) -> Result<(), (String, Option<PathBuf>)> {
    if manager::operation_fingerprint(path).ok().as_deref() != Some(&item.fingerprint) {
        return Err((
            "The installed folder changed after confirmation. Refresh and confirm again.".into(),
            None,
        ));
    }
    Ok(())
}

fn install_manage_one(
    item: &ManageItem,
    plan: &ManagePlan,
    settings_now: &Settings,
    stage: &Path,
    place: &impl Fn(&Path, &Path) -> Result<(), String>,
) -> Result<(), (String, Option<PathBuf>)> {
    let source = item.source.as_ref().expect("install source validated");
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    if manager::fingerprint(&source.source).ok().as_deref() != Some(&source.fingerprint) {
        return Err((
            "Source files changed after confirmation. Refresh and confirm again.".into(),
            None,
        ));
    }
    let destination = plan.destination_resolved.join(&item.folder_name);
    destination_is_absent(&destination).map_err(|error| (error, None))?;
    let staged = stage.join(&item.folder_name);
    fs::create_dir(&staged)
        .map_err(|error| (format!("Could not prepare staging folder: {error}"), None))?;
    manager::copy_materialized(
        &source.source,
        &staged,
        &[plan.destination_resolved.clone(), stage.to_path_buf()],
    )
    .map_err(|error| (error, None))?;
    if !staged_matches(&source.source, &staged).map_err(|error| (error, None))? {
        return Err((
            "Staged content did not match the source; nothing was installed.".into(),
            None,
        ));
    }
    if manager::fingerprint(&source.source).ok().as_deref() != Some(&source.fingerprint) {
        return Err((
            "Source files changed while staging. Refresh and confirm again.".into(),
            None,
        ));
    }
    ensure_plan_current(settings_now, plan).map_err(|error| (error, None))?;
    destination_is_absent(&destination).map_err(|error| (error, None))?;
    place(&staged, &destination).map_err(|error| {
        (
            format!("Could not place the prepared folder: {error}"),
            None,
        )
    })
}

fn action_verb(action: OperationAction) -> &'static str {
    match action {
        OperationAction::Install => "Installing",
        OperationAction::Update => "Updating",
        OperationAction::Uninstall => "Uninstalling",
    }
}
fn action_past(action: OperationAction) -> &'static str {
    match action {
        OperationAction::Install => "Installed",
        OperationAction::Update => "Updated",
        OperationAction::Uninstall => "Uninstalled",
    }
}

fn item_path(destination: &Path, installed: &InstalledBaseline) -> PathBuf {
    destination.join(installed.path.file_name().unwrap_or_default())
}

fn ensure_plan_current(settings_now: &Settings, plan: &ManagePlan) -> Result<(), String> {
    let destination = settings::configured_destination(settings_now, &plan.harness)
        .ok_or_else(|| "No destination is configured for this harness.".to_string())?;
    if !resolves_to(Some(destination), &plan.destination_resolved) {
        return Err(
            "Destination settings changed during the operation. Refresh and confirm again.".into(),
        );
    }
    if let Some(source_parent) = &plan.source_parent {
        if !resolves_to(settings_now.source.as_deref(), source_parent)
            || manager::paths_overlap(source_parent, &plan.destination_resolved)
        {
            return Err("Source or destination relationship changed during the operation. Refresh and confirm again.".into());
        }
    }
    Ok(())
}

fn ensure_uninstall_safe_from_source(
    settings_now: &Settings,
    installed_path: &Path,
) -> Result<(), String> {
    if let Some(source) = settings_now
        .source
        .as_deref()
        .and_then(|path| manager::resolved_path(path).ok())
    {
        if manager::paths_overlap_lexically(installed_path, &source) {
            return Err("The installed folder overlaps the configured source. Move the source or destination before uninstalling.".into());
        }
    }
    Ok(())
}

fn destination_is_absent(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "Could not verify destination {}: {error}",
            path.display()
        )),
        Ok(_) => Err("A folder with this name now exists; it was left untouched.".into()),
    }
}

fn recycle_to_system(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::PCWSTR,
            Win32::{
                System::Com::{
                    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL,
                    COINIT_APARTMENTTHREADED,
                },
                UI::Shell::{
                    FileOperation, IFileOperation, IShellItem, SHCreateItemFromParsingName,
                    FOFX_EARLYFAILURE, FOFX_RECYCLEONDELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
                    FOF_NOERRORUI, FOF_SILENT, FOF_WANTNUKEWARNING,
                },
            },
        };

        // The trash crate canonicalizes its input, which would turn a root junction into its external target.
        let absolute =
            manager::resolved_path(path.parent().ok_or("Cannot recycle a filesystem root.")?)?
                .join(
                    path.file_name()
                        .ok_or("Cannot recycle a filesystem root.")?,
                );
        let mut parsing_path = absolute.as_os_str().to_string_lossy().into_owned();
        if let Some(stripped) = parsing_path.strip_prefix(r"\\?\UNC\") {
            parsing_path = format!(r"\\{stripped}");
        } else if let Some(stripped) = parsing_path.strip_prefix(r"\\?\") {
            parsing_path = stripped.to_owned();
        }
        let wide: Vec<u16> = std::ffi::OsStr::new(&parsing_path)
            .encode_wide()
            .chain(Some(0))
            .collect();
        unsafe {
            let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if initialized.is_err() {
                return Err(format!(
                    "Could not initialize Windows shell recycling: {initialized}"
                ));
            }
            let result = (|| {
                let operation: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_ALL)
                    .map_err(|error| error.to_string())?;
                // FOFX_RECYCLEONDELETE forces recycling; EARLYFAILURE stops on unavailable/failed recycle instead of skipping.
                let flags = FOF_ALLOWUNDO
                    | FOF_NOCONFIRMATION
                    | FOF_NOERRORUI
                    | FOF_SILENT
                    | FOF_WANTNUKEWARNING
                    | FOFX_RECYCLEONDELETE
                    | FOFX_EARLYFAILURE;
                operation
                    .SetOperationFlags(flags)
                    .map_err(|error| error.to_string())?;
                let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
                    .map_err(|error| error.to_string())?;
                operation
                    .DeleteItem(&item, None)
                    .map_err(|error| error.to_string())?;
                operation
                    .PerformOperations()
                    .map_err(|error| error.to_string())?;
                if operation
                    .GetAnyOperationsAborted()
                    .map_err(|error| error.to_string())?
                    .as_bool()
                {
                    return Err("Windows aborted the recycle operation".to_string());
                }
                if fs::symlink_metadata(path).is_ok() {
                    return Err("Windows reported success, but the folder remains in place".into());
                }
                Ok(())
            })();
            CoUninitialize();
            result
        }
    }
    #[cfg(not(windows))]
    {
        trash::delete(path).map_err(|error| error.to_string())
    }
}

fn staged_matches(source: &Path, staged: &Path) -> Result<bool, String> {
    Ok(manager::content_fingerprint(source)? == manager::content_fingerprint(staged)?)
}

fn cleanup_stage(stage: &Path, destination: &Path) -> Result<(), String> {
    match fs::symlink_metadata(stage) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Could not inspect {}: {error}", stage.display())),
        Ok(_) => {}
    }
    if manager::is_link_or_reparse_path(stage)? || !manager::path_is_within(stage, destination) {
        return Err(format!(
            "Staging path no longer belongs to this operation: {}",
            stage.display()
        ));
    }
    fs::remove_dir_all(stage).map_err(|error| {
        format!(
            "Could not remove owned staging folder {}: {error}",
            stage.display()
        )
    })
}

fn resolves_to(path: Option<&Path>, captured: &Path) -> bool {
    path.and_then(|path| manager::resolved_path(path).ok())
        .is_some_and(|resolved| {
            manager::path_key_for_ops(&resolved) == manager::path_key_for_ops(captured)
        })
}

fn identity_key(name: &str) -> String {
    if cfg!(windows) {
        name.to_lowercase()
    } else {
        name.to_string()
    }
}

fn new_token() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "install-{now:x}-{:x}",
        NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
    )
}

fn new_stage_name() -> String {
    format!(
        "{STAGING_PREFIX}{}-{:x}",
        std::process::id(),
        NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
    )
}

fn scans() -> &'static Mutex<HashMap<String, ScanBaseline>> {
    SCANS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn plans() -> &'static Mutex<HashMap<String, ManagePlan>> {
    PLANS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn file(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::File::create(path).unwrap().write_all(contents).unwrap();
    }

    fn skill(root: &Path, name: &str, extra: &[u8]) -> SkillBaseline {
        let path = root.join(name);
        file(
            &path.join("SKILL.md"),
            format!("---\nname: {name}\n---\n").as_bytes(),
        );
        file(&path.join(".hidden/nested.txt"), extra);
        SkillBaseline {
            folder_name: name.into(),
            source: path.clone(),
            fingerprint: manager::fingerprint(&path).unwrap(),
            links: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[cfg(unix)]
    fn source_alias(target: &Path, alias: &Path) {
        std::os::unix::fs::symlink(target, alias).unwrap();
    }

    #[cfg(windows)]
    fn source_alias(target: &Path, alias: &Path) {
        let command = format!("mklink /J {} {}", alias.display(), target.display());
        let result = std::process::Command::new("cmd")
            .args(["/C", &command])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    fn remove_source_alias(alias: &Path) {
        #[cfg(unix)]
        std::fs::remove_file(alias).unwrap();
        #[cfg(windows)]
        std::fs::remove_dir(alias).unwrap();
    }

    #[test]
    fn scan_baseline_rejects_source_changes_before_confirmation() {
        let _test_guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let baseline_skill = skill(&source, "Changed", b"before");
        let settings_now = Settings {
            source: Some(source.clone()),
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let mut cache = manager::SourceCache::default();
        manager::scan_cached(&settings_now, "codex", &mut cache, false).unwrap();
        file(&baseline_skill.source.join(".hidden/nested.txt"), b"after");
        let response = manager::scan_cached(&settings_now, "codex", &mut cache, true).unwrap();
        let revision = response.revision.clone();
        let generation = begin_scan();
        record_scan(&response, generation);

        let error = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &revision,
            OperationAction::Install,
            &["Changed".into()],
        )
        .unwrap_err();
        assert!(error.contains("changed after the comparison"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn permission_changes_invalidate_scans_and_confirmations() {
        use std::os::unix::fs::PermissionsExt;
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let original = skill(&source, "One", b"same bytes");
        let script = original.source.join("run.sh");
        file(&script, b"#!/bin/sh\nexit 0\n");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let settings_now = Settings {
            source: Some(source),
            destinations: [("codex".to_string(), destination.clone())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        fs::set_permissions(&script, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(prepare_operation(
            &settings_now,
            "codex".into(),
            &response.revision,
            OperationAction::Install,
            &["One".into()]
        )
        .is_err());
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            "codex".into(),
            &response.revision,
            OperationAction::Install,
            &["One".into()],
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let mut events = Vec::new();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |_| panic!("Install must not recycle"),
            |event| events.push(event),
        )
        .unwrap();
        assert!(!destination.join("One").exists());
        assert!(events.iter().any(|event| event.success == Some(false)));
        let installed = skill(&destination, "One", b"installed");
        let installed_file = installed.source.join("SKILL.md");
        fs::set_permissions(&installed_file, fs::Permissions::from_mode(0o644)).unwrap();
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            "codex".into(),
            &response.revision,
            OperationAction::Uninstall,
            &["One".into()],
        )
        .unwrap();
        fs::set_permissions(&installed_file, fs::Permissions::from_mode(0o600)).unwrap();
        let mut events = Vec::new();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |_| panic!("Changed permissions must prevent recycling"),
            |event| events.push(event),
        )
        .unwrap();
        assert!(installed_file.exists());
        assert!(events.iter().any(|event| event.success == Some(false)));
    }

    #[test]
    fn retargeted_source_alias_is_rejected_even_when_contents_match() {
        let _test_guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let alias = temp.path().join("source-alias");
        let destination = temp.path().join("destination");
        let _one = skill(&first, "Same", b"identical bytes");
        let _two = skill(&second, "Same", b"identical bytes");
        source_alias(&first, &alias);
        let settings_now = Settings {
            source: Some(alias.clone()),
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        let revision = response.revision.clone();
        record_scan(&response, begin_scan());
        remove_source_alias(&alias);
        source_alias(&second, &alias);

        let error = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &revision,
            OperationAction::Install,
            &["Same".into()],
        )
        .unwrap_err();
        assert!(error.contains("changed or overlaps after this scan"));
    }

    #[test]
    fn staged_corruption_is_detected_and_cleanup_stays_in_owned_directory() {
        let _test_guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let stage = destination.join(".ai-skill-manager-staging-owned");
        let outside = temp.path().join("unrelated");
        let skill = skill(&source, "One", b"original");
        fs::create_dir_all(&stage).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let staged = stage.join("One");
        fs::create_dir(&staged).unwrap();
        manager::copy_materialized(&skill.source, &staged, std::slice::from_ref(&destination))
            .unwrap();
        assert!(staged_matches(&skill.source, &staged).unwrap());
        file(&staged.join(".hidden/nested.txt"), b"corrupted");
        assert!(!staged_matches(&skill.source, &staged).unwrap());
        file(&outside.join("keep.txt"), b"unrelated");

        cleanup_stage(&stage, &destination).unwrap();
        assert!(!stage.exists());
        assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"unrelated");
    }

    #[test]
    fn confirmed_token_is_single_use_and_partial_failures_preserve_inputs() {
        let _test_guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let changed = skill(&source, "changed", b"before");
        skill(&source, "competing", b"source bytes");
        let good = skill(&source, "good", b"copied bytes");
        fs::create_dir(good.source.join("empty")).unwrap();
        let external = temp.path().join("external");
        file(&external.join("data.txt"), b"external bytes");
        source_alias(&external, &good.source.join("resources"));
        let settings_now = Settings {
            source: Some(source),
            destinations: [("codex".to_string(), destination.clone())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let selected = vec![
            "changed".into(),
            "competing".into(),
            "good".into(),
            "good".into(),
            "unknown".into(),
        ];
        let cancelled = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Install,
            &selected,
        )
        .unwrap();
        assert_eq!(cancelled.eligible.len(), 3);
        assert_eq!(cancelled.skipped.len(), 1);
        assert!(!destination.exists());
        assert!(
            execute_operation(&cancelled.token, &settings_now, false, |_| Ok(()), |_| {}).is_err()
        );
        assert!(!destination.exists());
        let confirmed = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Install,
            &selected,
        )
        .unwrap();
        file(
            &changed.source.join(".hidden/nested.txt"),
            b"after confirmation",
        );
        file(&destination.join("competing/keep.txt"), b"untouched");
        let mut events = Vec::new();
        execute_operation(
            &confirmed.token,
            &settings_now,
            true,
            |_| Ok(()),
            |event| events.push(event),
        )
        .unwrap();
        let results: Vec<_> = events
            .iter()
            .filter(|event| event.kind == "result")
            .map(|event| event.success)
            .collect();
        assert_eq!(results, vec![Some(false), Some(false), Some(true)]);
        assert!(!destination.join("changed").exists());
        assert_eq!(
            fs::read(destination.join("competing/keep.txt")).unwrap(),
            b"untouched"
        );
        assert_eq!(
            fs::read(destination.join("good/.hidden/nested.txt")).unwrap(),
            b"copied bytes"
        );
        assert!(destination.join("good/empty").is_dir());
        assert!(!manager::is_link_or_reparse_path(&destination.join("good/resources")).unwrap());
        assert_eq!(
            fs::read(external.join("data.txt")).unwrap(),
            b"external bytes"
        );
        assert_eq!(
            manager::content_fingerprint(&good.source).unwrap(),
            manager::content_fingerprint(&destination.join("good")).unwrap()
        );
        assert!(fs::read_dir(&destination).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(STAGING_PREFIX)));
        assert_eq!(events.last().unwrap().kind, "finished");
        assert!(
            execute_operation(&confirmed.token, &settings_now, true, |_| Ok(()), |_| {}).is_err()
        );
    }

    #[test]
    fn rescanning_invalidates_backend_held_revision_and_execution_is_serial() {
        let _test_guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let _skill = skill(&source, "One", b"content");
        let settings_now = Settings {
            source: Some(source),
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        let revision = response.revision.clone();
        record_scan(&response, begin_scan());
        begin_scan();
        assert!(prepare_operation(
            &settings_now,
            "codex".to_string(),
            &revision,
            OperationAction::Install,
            &["One".into()]
        )
        .is_err());

        let _guard = acquire().unwrap();
        assert!(acquire().is_err());
    }

    #[test]
    fn invalid_source_keeps_identifiable_installed_skill_uninstallable_without_source() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(source.join("Broken")).unwrap();
        let installed = skill(&destination, "Broken", b"installed");
        let settings_now = Settings {
            source: Some(source),
            destinations: [("codex".to_string(), destination.clone())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        let row = &response.skills[0];
        assert_eq!(row.status, "invalid_source");
        assert!(row
            .eligible_actions
            .iter()
            .any(|action| action == "uninstall"));
        record_scan(&response, begin_scan());
        let no_source = Settings {
            source: None,
            destinations: settings_now.destinations.clone(),
            ..Settings::default()
        };
        let prepared = prepare_operation(
            &no_source,
            "codex".to_string(),
            &response.revision,
            OperationAction::Uninstall,
            &["Broken".into()],
        )
        .unwrap();
        assert!(prepared.eligible[0]
            .warnings
            .iter()
            .any(|warning| warning == row.error.as_ref().unwrap()));
        let mut events = Vec::new();
        let recycled = temp.path().join("recycled");
        execute_operation(
            &prepared.token,
            &no_source,
            true,
            |path| fs::rename(path, &recycled).map_err(|error| error.to_string()),
            |event| events.push(event),
        )
        .unwrap();
        assert!(!installed.source.exists());
        assert!(events.iter().any(|event| event.success == Some(true)));
    }

    #[test]
    fn stale_installed_baseline_rejects_uninstall() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let installed = skill(&destination, "One", b"before");
        let settings_now = Settings {
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        file(&installed.source.join(".hidden/nested.txt"), b"after");
        let error = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Uninstall,
            &["One".into()],
        )
        .unwrap_err();
        assert!(error.contains("changed after the comparison"));
    }

    #[test]
    fn source_destination_overlap_blocks_uninstall() {
        let _guard = test_lock();
        for source_inside_skill in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let destination = temp.path().join("destination");
            let installed = skill(&destination, "One", b"content");
            let source = if source_inside_skill {
                installed.source.join(".hidden")
            } else {
                destination.clone()
            };
            fs::create_dir_all(&source).unwrap();
            let settings_now = Settings {
                source: Some(source),
                destinations: [("codex".to_string(), destination)].into_iter().collect(),
                ..Settings::default()
            };
            let response = manager::scan(&settings_now, "codex").unwrap();
            record_scan(&response, begin_scan());
            let error = prepare_operation(
                &settings_now,
                "codex".to_string(),
                &response.revision,
                OperationAction::Uninstall,
                &["One".into()],
            )
            .unwrap_err();
            assert!(error.contains("overlaps the configured source"));
        }
    }

    #[test]
    fn destination_change_after_confirmation_prevents_recycling() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let installed = skill(&destination, "One", b"before");
        let settings_now = Settings {
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Uninstall,
            &["One".into()],
        )
        .unwrap();
        file(&installed.source.join(".hidden/nested.txt"), b"changed");
        let mut recycled = false;
        let mut events = Vec::new();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |_| {
                recycled = true;
                Ok(())
            },
            |event| events.push(event),
        )
        .unwrap();
        assert!(!recycled);
        assert_eq!(
            fs::read(installed.source.join(".hidden/nested.txt")).unwrap(),
            b"changed"
        );
        assert!(events.iter().any(|event| event.success == Some(false)));
    }

    #[test]
    fn recycle_failure_leaves_installed_folder_untouched() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let installed = skill(&destination, "One", b"preserve");
        let settings_now = Settings {
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Uninstall,
            &["One".into()],
        )
        .unwrap();
        let mut events = Vec::new();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |_| Err("recycle unavailable".into()),
            |event| events.push(event),
        )
        .unwrap();
        assert_eq!(
            fs::read(installed.source.join(".hidden/nested.txt")).unwrap(),
            b"preserve"
        );
        assert!(events.iter().any(|event| event.success == Some(false)
            && event
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("system trash")));
    }

    #[test]
    fn placement_failure_after_recycle_retains_prepared_copy_and_reports_paths() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let old = skill(&destination, "One", b"old");
        skill(&source, "One", b"new");
        let settings_now = Settings {
            source: Some(source),
            destinations: [("codex".to_string(), destination.clone())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Update,
            &["One".into()],
        )
        .unwrap();
        let recycle = temp.path().join("recycled");
        let mut events = Vec::new();
        execute_operation_with_place(
            &prepared.token,
            &settings_now,
            true,
            |path| {
                fs::rename(path, &recycle).map_err(|error| error.to_string())?;
                fs::create_dir(path).map_err(|error| error.to_string())
            },
            |_, _| Err("forced placement failure".into()),
            |event| events.push(event),
        )
        .unwrap();
        assert!(
            !old.source.join(".hidden/nested.txt").exists(),
            "{events:?}"
        );
        assert_eq!(
            fs::read(recycle.join(".hidden/nested.txt")).unwrap(),
            b"old"
        );
        let failure = events
            .iter()
            .find(|event| event.kind == "result")
            .unwrap()
            .message
            .as_deref()
            .unwrap();
        let prepared_path = failure
            .split("Prepared copy retained at ")
            .nth(1)
            .unwrap()
            .split(". Restore")
            .next()
            .unwrap();
        assert!(Path::new(prepared_path).is_dir(), "{failure}");
        assert!(failure.contains("Restore the original manually from the system trash"));
    }

    #[test]
    fn update_replaces_complete_tree_and_continues_after_one_stale_skill() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let old_one = skill(&destination, "One", b"old one");
        let old_two = skill(&destination, "Two", b"old two");
        let new_one = skill(&source, "One", b"new one");
        let new_two = skill(&source, "Two", b"new two");
        file(&old_one.source.join("obsolete.txt"), b"remove me");
        fs::create_dir(new_one.source.join("empty")).unwrap();
        let settings_now = Settings {
            source: Some(source),
            destinations: [("codex".to_string(), destination.clone())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let selected = vec!["One".into(), "Two".into()];
        let prepared = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Update,
            &selected,
        )
        .unwrap();
        file(
            &new_two.source.join(".hidden/nested.txt"),
            b"changed after prepare",
        );
        let recycle_root = temp.path().join("recycled");
        fs::create_dir(&recycle_root).unwrap();
        let mut events = Vec::new();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |path| {
                fs::rename(path, recycle_root.join(path.file_name().unwrap()))
                    .map_err(|error| error.to_string())
            },
            |event| events.push(event),
        )
        .unwrap();
        let results: Vec<_> = events
            .iter()
            .filter(|event| event.kind == "result")
            .map(|event| event.success)
            .collect();
        assert_eq!(results, vec![Some(true), Some(false)]);
        assert_eq!(
            fs::read(destination.join("One/.hidden/nested.txt")).unwrap(),
            b"new one"
        );
        assert!(destination.join("One/empty").is_dir());
        assert!(!destination.join("One/obsolete.txt").exists());
        assert_eq!(
            fs::read(recycle_root.join("One/obsolete.txt")).unwrap(),
            b"remove me"
        );
        assert_eq!(
            fs::read(old_two.source.join(".hidden/nested.txt")).unwrap(),
            b"old two"
        );
        assert_eq!(
            fs::read(new_two.source.join(".hidden/nested.txt")).unwrap(),
            b"changed after prepare"
        );
    }

    #[test]
    fn custom_harness_supports_install_and_removed_registry_invalidates_confirmation() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("custom-skills");
        let first = skill(&source, "First", b"first");
        fs::create_dir(&destination).unwrap();
        let id = "custom-550e8400-e29b-41d4-a716-446655440000";
        let settings_now = Settings {
            source: Some(source.clone()),
            destinations: [(id.to_string(), destination.clone())]
                .into_iter()
                .collect(),
            custom_harnesses: [(id.to_string(), "Research".to_string())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, id).unwrap();
        assert_eq!(response.skills[0].status, "missing");
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            id.to_string(),
            &response.revision,
            OperationAction::Install,
            &["First".into()],
        )
        .unwrap();
        execute_operation(&prepared.token, &settings_now, true, |_| Ok(()), |_| {}).unwrap();
        assert_eq!(
            fs::read(destination.join("First/.hidden/nested.txt")).unwrap(),
            b"first"
        );

        file(&first.source.join(".hidden/nested.txt"), b"updated");
        let response = manager::scan(&settings_now, id).unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            id.to_string(),
            &response.revision,
            OperationAction::Update,
            &["First".into()],
        )
        .unwrap();
        let recycle_root = temp.path().join("recycled");
        fs::create_dir(&recycle_root).unwrap();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |path| {
                fs::rename(path, recycle_root.join(path.file_name().unwrap()))
                    .map_err(|error| error.to_string())
            },
            |_| {},
        )
        .unwrap();
        assert_eq!(
            fs::read(destination.join("First/.hidden/nested.txt")).unwrap(),
            b"updated"
        );

        let response = manager::scan(&settings_now, id).unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            id.to_string(),
            &response.revision,
            OperationAction::Uninstall,
            &["First".into()],
        )
        .unwrap();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |path| {
                fs::rename(path, recycle_root.join("uninstalled-First"))
                    .map_err(|error| error.to_string())
            },
            |_| {},
        )
        .unwrap();
        assert!(!destination.join("First").exists());
        assert!(recycle_root.join("uninstalled-First").is_dir());

        let _second = skill(&source, "Second", b"second");
        let response = manager::scan(&settings_now, id).unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            id.to_string(),
            &response.revision,
            OperationAction::Install,
            &["Second".into()],
        )
        .unwrap();
        let other_destination = temp.path().join("changed-destination");
        fs::create_dir(&other_destination).unwrap();
        let changed_path = Settings {
            destinations: [(id.to_string(), other_destination)].into_iter().collect(),
            ..settings_now.clone()
        };
        assert!(prepare_operation(
            &changed_path,
            id.to_string(),
            &response.revision,
            OperationAction::Install,
            &["Second".into()]
        )
        .unwrap_err()
        .contains("Destination settings changed"));
        let mut removed = settings_now.clone();
        removed.custom_harnesses.remove(id);
        removed.destinations.remove(id);
        assert!(
            execute_operation(&prepared.token, &removed, true, |_| Ok(()), |_| {})
                .unwrap_err()
                .contains("No destination")
        );
        assert!(!destination.join("Second").exists());
        assert!(first.source.exists());
    }

    #[test]
    fn recycling_junction_entry_does_not_follow_external_target() {
        let _guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let external = temp.path().join("external");
        let bin = temp.path().join("recycled");
        let installed = skill(&destination, "One", b"installed");
        file(&external.join("keep.txt"), b"external");
        source_alias(&external, &installed.source.join("resources"));
        let settings_now = Settings {
            destinations: [("codex".to_string(), destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, "codex").unwrap();
        record_scan(&response, begin_scan());
        let prepared = prepare_operation(
            &settings_now,
            "codex".to_string(),
            &response.revision,
            OperationAction::Uninstall,
            &["One".into()],
        )
        .unwrap();
        execute_operation(
            &prepared.token,
            &settings_now,
            true,
            |path| fs::rename(path, &bin).map_err(|error| error.to_string()),
            |_| {},
        )
        .unwrap();
        assert_eq!(fs::read(external.join("keep.txt")).unwrap(), b"external");
        assert!(bin.join("resources").exists());
    }
}
