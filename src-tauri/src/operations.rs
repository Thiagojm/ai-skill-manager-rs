use crate::manager::{self, Harness, LinkWarning, ScanResponse};
use crate::settings::{self, Settings};
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
static PLANS: OnceLock<Mutex<HashMap<String, InstallPlan>>> = OnceLock::new();

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
}

#[derive(Clone)]
struct InstallPlan {
    harness: Harness,
    source_setting: PathBuf,
    source_parent: PathBuf,
    destination_resolved: PathBuf,
    destination_setting: PathBuf,
    skills: Vec<SkillBaseline>,
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
            if row.status != "missing" {
                return None;
            }
            Some((
                identity_key(&row.folder_name),
                SkillBaseline {
                    folder_name: row.folder_name.clone(),
                    source: scan.resolved_source_path.as_ref()?.join(&row.folder_name),
                    fingerprint: row.source_fingerprint.clone()?,
                    links: row.link_warnings.clone(),
                    warnings: row.warnings.clone(),
                },
            ))
        })
        .collect();
    scans().lock().unwrap_or_else(|e| e.into_inner()).insert(
        scan.revision.clone(),
        ScanBaseline {
            harness: scan.harness,
            source_parent: scan.resolved_source_path.clone(),
            destination_resolved: scan.resolved_destination_path.clone(),
            warnings: scan.warnings.clone(),
            skills,
        },
    );
}

pub fn prepare(
    settings_now: &Settings,
    harness: Harness,
    revision: &str,
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
    let source_parent = baseline.source_parent.clone().ok_or_else(|| {
        "Choose an accessible source folder before installing skills.".to_string()
    })?;
    let destination = settings::configured_destination(settings_now, harness)
        .map(Path::to_path_buf)
        .ok_or_else(|| "No destination is configured for this harness.".to_string())?;
    let destination_resolved = manager::resolved_path(&destination)?;
    if !resolves_to(settings_now.source.as_deref(), &source_parent)
        || manager::path_key_for_ops(&destination_resolved)
            != manager::path_key_for_ops(&baseline.destination_resolved)
    {
        return Err(
            "Source or destination settings changed after this scan. Refresh and confirm again."
                .into(),
        );
    }
    if manager::paths_overlap(&source_parent, &destination_resolved) {
        return Err(
            "Source and destination overlap. Choose separate folders before installing.".into(),
        );
    }

    let mut eligible = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in selected {
        let key = identity_key(name);
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(skill) = baseline.skills.get(&key).cloned() else {
            skipped.push(format!("{name} (not eligible in this scan)"));
            continue;
        };
        if manager::paths_overlap(&skill.source, &destination_resolved) {
            return Err(format!(
                "Source skill {} overlaps the destination.",
                skill.folder_name
            ));
        }
        for warning in &skill.links {
            let target = PathBuf::from(&warning.target);
            if manager::paths_overlap(&target, &destination_resolved) {
                return Err(format!(
                    "Link in {} targets the destination tree.",
                    skill.folder_name
                ));
            }
        }
        if manager::fingerprint(&skill.source)? != skill.fingerprint {
            return Err(format!(
                "{} changed after the comparison. Refresh and confirm again.",
                skill.folder_name
            ));
        }
        if fs::symlink_metadata(destination_resolved.join(&skill.folder_name)).is_ok() {
            return Err(format!(
                "{} already exists at the destination. Refresh and confirm again.",
                skill.folder_name
            ));
        }
        eligible.push(skill);
    }
    if eligible.is_empty() {
        return Err("None of the selected skills is eligible to install.".into());
    }
    let warnings = eligible
        .iter()
        .flat_map(|skill| skill.links.clone())
        .collect();
    let notices = baseline.warnings;
    let token = new_token();
    let mut pending = plans().lock().unwrap_or_else(|e| e.into_inner());
    pending.clear();
    pending.insert(
        token.clone(),
        InstallPlan {
            harness,
            source_setting: settings_now.source.clone().expect("source validated above"),
            source_parent,
            destination_resolved: destination_resolved.clone(),
            destination_setting: destination.clone(),
            skills: eligible.clone(),
            warnings,
        },
    );
    Ok(PrepareResponse {
        token,
        eligible: eligible
            .iter()
            .map(|skill| PreparedSkill {
                folder_name: skill.folder_name.clone(),
                link_warnings: skill.links.clone(),
                warnings: skill.warnings.clone(),
            })
            .collect(),
        skipped,
        destination_path: destination,
        warnings: notices,
    })
}

pub fn execute_locked(
    token: &str,
    settings_now: &Settings,
    acknowledge_links: bool,
    channel: Channel<OperationEvent>,
) -> Result<(), String> {
    RUNNING.store(true, Ordering::Release);
    let result = execute_plan(token, settings_now, acknowledge_links, |event| {
        let _ = channel.send(event);
    });
    RUNNING.store(false, Ordering::Release);
    result
}

fn execute_plan(
    token: &str,
    settings_now: &Settings,
    acknowledge_links: bool,
    mut emit: impl FnMut(OperationEvent),
) -> Result<(), String> {
    let plan = plans()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(token)
        .ok_or_else(|| {
            "This installation confirmation expired. Refresh and confirm again.".to_string()
        })?;
    ensure_link_acknowledgement(&plan, acknowledge_links)?;
    let configured = settings::configured_destination(settings_now, plan.harness)
        .ok_or_else(|| "No destination is configured for this harness.".to_string())?;
    if !resolves_to(settings_now.source.as_deref(), &plan.source_parent)
        || !resolves_to(Some(configured), &plan.destination_resolved)
    {
        return Err(
            "Source or destination settings changed after confirmation. Refresh and confirm again."
                .into(),
        );
    }
    fs::create_dir_all(&plan.destination_resolved).map_err(|error| {
        format!(
            "Could not create destination {}: {error}",
            plan.destination_resolved.display()
        )
    })?;
    let stage = plan.destination_resolved.join(new_stage_name());
    fs::create_dir(&stage).map_err(|error| format!("Could not create staging folder: {error}"))?;
    let completed = run_batch(&plan, &stage, &mut emit);
    let cleanup = cleanup_stage(&stage, &plan.destination_resolved);
    let cleanup_ok = cleanup.is_ok();
    emit(OperationEvent {
        kind: "finished".into(),
        folder_name: None,
        completed,
        total: plan.skills.len(),
        success: Some(cleanup_ok),
        message: Some(match cleanup {
            Ok(()) => "Installation batch finished.".into(),
            Err(error) => {
                format!("Installation finished, but staging cleanup needs attention: {error}")
            }
        }),
    });
    Ok(())
}

fn run_batch(plan: &InstallPlan, stage: &Path, mut emit: impl FnMut(OperationEvent)) -> usize {
    let total = plan.skills.len();
    let mut completed = 0;
    for skill in &plan.skills {
        emit(OperationEvent {
            kind: "progress".into(),
            folder_name: Some(skill.folder_name.clone()),
            completed,
            total,
            success: None,
            message: Some(format!("Preparing {}", skill.folder_name)),
        });
        let result = install_one(plan, skill, stage);
        completed += 1;
        emit(OperationEvent {
            kind: "result".into(),
            folder_name: Some(skill.folder_name.clone()),
            completed,
            total,
            success: Some(result.is_ok()),
            message: Some(match result {
                Ok(()) => "Installed successfully.".into(),
                Err(error) => error,
            }),
        });
    }
    completed
}

fn install_one(plan: &InstallPlan, skill: &SkillBaseline, stage: &Path) -> Result<(), String> {
    if !resolves_to(Some(&plan.source_setting), &plan.source_parent)
        || !resolves_to(Some(&plan.destination_setting), &plan.destination_resolved)
        || manager::paths_overlap(&skill.source, &plan.destination_resolved)
    {
        return Err(
            "Source or destination relationship changed; refresh and confirm again.".into(),
        );
    }
    if manager::fingerprint(&skill.source)? != skill.fingerprint {
        return Err("Source files changed after confirmation. Refresh and confirm again.".into());
    }
    let destination = plan.destination_resolved.join(&skill.folder_name);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err("A folder with this name now exists; it was left untouched.".into());
    }
    let staged = stage.join(&skill.folder_name);
    fs::create_dir(&staged)
        .map_err(|error| format!("Could not prepare staging folder: {error}"))?;
    manager::copy_materialized(
        &skill.source,
        &staged,
        &[plan.destination_resolved.clone(), stage.to_path_buf()],
    )?;
    if !staged_matches(&skill.source, &staged)? {
        return Err("Staged content did not match the source; nothing was installed.".into());
    }
    if manager::fingerprint(&skill.source)? != skill.fingerprint {
        return Err("Source files changed while staging. Refresh and confirm again.".into());
    }
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(
            "A folder with this name appeared during installation; it was left untouched.".into(),
        );
    }
    fs::rename(&staged, &destination)
        .map_err(|error| format!("Could not place the prepared folder: {error}"))
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

fn ensure_link_acknowledgement(plan: &InstallPlan, acknowledged: bool) -> Result<(), String> {
    if !plan.warnings.is_empty() && !acknowledged {
        Err("Acknowledge the listed links and junctions before continuing.".into())
    } else {
        Ok(())
    }
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

fn plans() -> &'static Mutex<HashMap<String, InstallPlan>> {
    PLANS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
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
    fn batch_materializes_complete_tree_and_keeps_competing_folder() {
        let _test_guard = test_lock();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&destination).unwrap();
        let first = skill(&source, "first", b"nested content");
        let second = skill(&source, "second", b"other content");
        let third = skill(&source, "third", b"will change");
        let plan = InstallPlan {
            harness: Harness::Codex,
            source_setting: source.clone(),
            source_parent: manager::resolved_path(&source).unwrap(),
            destination_resolved: manager::resolved_path(&destination).unwrap(),
            destination_setting: destination.clone(),
            skills: vec![first, second, third.clone()],
            warnings: Vec::new(),
        };
        assert!(!manager::paths_overlap(
            &plan.skills[0].source,
            &destination
        ));
        file(&destination.join("second/keep.txt"), b"untouched");
        file(
            &third.source.join(".hidden/nested.txt"),
            b"changed after confirmation",
        );
        let stage = destination.join(".ai-skill-manager-staging-test");
        fs::create_dir(&stage).unwrap();
        let mut events = Vec::new();
        let completed = run_batch(&plan, &stage, |event| events.push(event));

        assert_eq!(completed, 3);
        assert_eq!(events[1].success, Some(true), "{events:?}");
        assert_eq!(
            fs::read(destination.join("first/.hidden/nested.txt")).unwrap(),
            b"nested content"
        );
        assert_eq!(
            fs::read(destination.join("second/keep.txt")).unwrap(),
            b"untouched"
        );
        assert!(!destination.join("second/.hidden/nested.txt").exists());
        assert!(!destination.join("third").exists());
        assert!(events
            .iter()
            .any(|event| event.folder_name.as_deref() == Some("first")
                && event.success == Some(true)));
        assert!(events
            .iter()
            .any(|event| event.folder_name.as_deref() == Some("second")
                && event.success == Some(false)));
        assert!(events
            .iter()
            .any(|event| event.folder_name.as_deref() == Some("third")
                && event.success == Some(false)));
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
            destinations: [(Harness::Codex, destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, Harness::Codex).unwrap();
        let revision = response.revision.clone();
        let generation = begin_scan();
        record_scan(&response, generation);
        file(&baseline_skill.source.join(".hidden/nested.txt"), b"after");

        let error = prepare(
            &settings_now,
            Harness::Codex,
            &revision,
            &["Changed".into()],
        )
        .unwrap_err();
        assert!(error.contains("changed after the comparison"));
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
            destinations: [(Harness::Codex, destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, Harness::Codex).unwrap();
        let revision = response.revision.clone();
        record_scan(&response, begin_scan());
        remove_source_alias(&alias);
        source_alias(&second, &alias);

        let error =
            prepare(&settings_now, Harness::Codex, &revision, &["Same".into()]).unwrap_err();
        assert!(error.contains("settings changed after this scan"));
    }

    #[test]
    fn linked_content_requires_explicit_acknowledgement() {
        let _test_guard = test_lock();
        let warning = LinkWarning {
            path: "linked".into(),
            target: "C:/outside".into(),
        };
        let plan = InstallPlan {
            harness: Harness::Codex,
            source_setting: PathBuf::new(),
            source_parent: PathBuf::new(),
            destination_resolved: PathBuf::new(),
            destination_setting: PathBuf::new(),
            skills: Vec::new(),
            warnings: vec![warning],
        };
        assert!(ensure_link_acknowledgement(&plan, false).is_err());
        assert!(ensure_link_acknowledgement(&plan, true).is_ok());
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
            destinations: [(Harness::Codex, destination.clone())]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, Harness::Codex).unwrap();
        record_scan(&response, begin_scan());
        let selected = vec![
            "changed".into(),
            "competing".into(),
            "good".into(),
            "good".into(),
            "unknown".into(),
        ];
        let cancelled =
            prepare(&settings_now, Harness::Codex, &response.revision, &selected).unwrap();
        assert_eq!(cancelled.eligible.len(), 3);
        assert_eq!(cancelled.skipped.len(), 1);
        assert!(!destination.exists());
        assert!(execute_plan(&cancelled.token, &settings_now, false, |_| {}).is_err());
        assert!(!destination.exists());
        let confirmed =
            prepare(&settings_now, Harness::Codex, &response.revision, &selected).unwrap();
        file(
            &changed.source.join(".hidden/nested.txt"),
            b"after confirmation",
        );
        file(&destination.join("competing/keep.txt"), b"untouched");
        let mut events = Vec::new();
        execute_plan(&confirmed.token, &settings_now, true, |event| {
            events.push(event)
        })
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
        assert!(execute_plan(&confirmed.token, &settings_now, true, |_| {}).is_err());
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
            destinations: [(Harness::Codex, destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = manager::scan(&settings_now, Harness::Codex).unwrap();
        let revision = response.revision.clone();
        record_scan(&response, begin_scan());
        begin_scan();
        assert!(prepare(&settings_now, Harness::Codex, &revision, &["One".into()]).is_err());

        let _guard = acquire().unwrap();
        assert!(acquire().is_err());
    }
}
