use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::settings::{self, Settings};

static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Harness {
    Codex,
    Claude,
    Antigravity,
    OpenCode,
}

impl Harness {
    pub const ALL: [Self; 4] = [Self::Codex, Self::Claude, Self::Antigravity, Self::OpenCode];

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
            Self::Antigravity => "Antigravity IDE",
            Self::OpenCode => "OpenCode",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRow {
    pub folder_name: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub source_path: Option<PathBuf>,
    pub destination_path: Option<PathBuf>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
    pub differences: Vec<Difference>,
    pub link_warnings: Vec<LinkWarning>,
    #[serde(skip)]
    pub(crate) source_fingerprint: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Difference {
    pub path: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkWarning {
    pub path: String,
    pub target: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResponse {
    pub revision: String,
    pub harness: Harness,
    pub source_path: Option<PathBuf>,
    pub destination_path: PathBuf,
    pub skills: Vec<SkillRow>,
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub(crate) resolved_source_path: Option<PathBuf>,
    #[serde(skip)]
    pub(crate) resolved_destination_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
struct Metadata {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Entry {
    Directory,
    File([u8; 32]),
    Link {
        target_is_dir: bool,
        content: Option<[u8; 32]>,
    },
}

type Inventory = BTreeMap<String, Entry>;

#[derive(Debug, Default)]
struct TreeScan {
    entries: Inventory,
    links: Vec<LinkWarning>,
}

#[derive(Default)]
struct SkillInfo {
    path: PathBuf,
    name: Option<String>,
    description: Option<String>,
    warnings: Vec<String>,
    metadata_error: Option<String>,
    ambiguous: bool,
}

pub fn scan(settings: &Settings, harness: Harness) -> Result<ScanResponse, String> {
    let source = settings.source.clone();
    if let Some(path) = &source {
        if !path.is_dir() {
            return Err(format!("Source folder is unavailable: {}", path.display()));
        }
    }
    let destination = settings::configured_destination(settings, harness)
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("No destination is configured for {}", harness.label()))?;
    let resolved_source_path = source.as_deref().map(resolve_path).transpose()?;
    let resolved_destination_path = resolve_path(&destination)?;
    let mut warnings = Vec::new();
    let mut source_skills = if let Some(path) = &source {
        read_source(path)?
    } else {
        BTreeMap::new()
    };
    let mut installed = read_installed(&destination)?;

    let keys: BTreeSet<String> = source_skills
        .keys()
        .chain(installed.keys())
        .cloned()
        .collect();
    let mut skills = Vec::with_capacity(keys.len());
    for key in keys {
        let source_skill = source_skills.remove(&key);
        let installed_skill = installed.remove(&key);
        let info = source_skill.as_ref().or(installed_skill.as_ref());
        let folder_name = info
            .map(|s| s.path.file_name().unwrap().to_string_lossy().into_owned())
            .unwrap_or(key);
        let source_path = source_skill.as_ref().map(|s| s.path.clone());
        let destination_path = installed_skill.as_ref().map(|s| s.path.clone());
        let mut row = SkillRow {
            folder_name: folder_name.clone(),
            name: None,
            description: None,
            status: String::new(),
            source_path: source_path.clone(),
            destination_path: destination_path.clone(),
            warnings: Vec::new(),
            error: None,
            differences: Vec::new(),
            link_warnings: Vec::new(),
            source_fingerprint: None,
        };

        if source_skill.as_ref().is_some_and(|s| s.ambiguous)
            || installed_skill.as_ref().is_some_and(|s| s.ambiguous)
        {
            row.status = "ambiguous".into();
            row.error = Some("More than one folder has this Windows-insensitive identity. Resolve the name collision before managing it.".into());
            skills.push(row);
            continue;
        }

        if let Some(source_skill) = &source_skill {
            row.name = source_skill.name.clone();
            row.description = source_skill.description.clone();
            row.warnings.extend(source_skill.warnings.clone());
            if let Some(error) = &source_skill.metadata_error {
                row.status = "invalid_source".into();
                row.error = Some(error.clone());
                skills.push(row);
                continue;
            }
        } else if let Some(installed_skill) = &installed_skill {
            row.name = installed_skill.name.clone();
            row.description = installed_skill.description.clone();
            row.warnings.extend(installed_skill.warnings.clone());
        }
        if source_skill.is_some() {
            if let Some(installed_skill) = &installed_skill {
                for warning in &installed_skill.warnings {
                    if !row.warnings.contains(warning) {
                        row.warnings.push(warning.clone());
                    }
                }
            }
        }

        let source_tree = source_skill.as_ref().map(|skill| inventory(&skill.path));
        let installed_tree = installed_skill.as_ref().map(|skill| inventory(&skill.path));
        match (&source_skill, &installed_skill) {
            (Some(_), Some(_)) => match (source_tree.unwrap(), installed_tree.unwrap()) {
                (Ok(source_tree), Ok(installed_tree)) => {
                    row.source_fingerprint = Some(tree_fingerprint(&source_tree));
                    let (equal, differences) = compare_trees(&source_tree, &installed_tree);
                    row.status = if equal { "identical" } else { "different" }.into();
                    row.differences = differences;
                    row.link_warnings.extend(source_tree.links);
                    row.link_warnings.extend(installed_tree.links);
                }
                (Err(error), _) | (_, Err(error)) => {
                    row.status = "error".into();
                    row.error = Some(error);
                }
            },
            (Some(_), None) => match source_tree.unwrap() {
                Ok(tree) => {
                    row.status = "missing".into();
                    row.source_fingerprint = Some(tree_fingerprint(&tree));
                    row.link_warnings = tree.links;
                }
                Err(error) => {
                    row.status = "error".into();
                    row.error = Some(error);
                }
            },
            (None, Some(_)) => match installed_tree.unwrap() {
                Ok(tree) => {
                    row.status = "installed_only".into();
                    row.link_warnings = tree.links;
                }
                Err(error) => {
                    row.status = "error".into();
                    row.error = Some(error);
                }
            },
            (None, None) => unreachable!(),
        }
        skills.push(row);
    }

    if !destination.exists() {
        warnings.push("The configured destination folder does not exist yet.".into());
    }
    let shared: Vec<_> = Harness::ALL
        .into_iter()
        .filter(|other| *other != harness)
        .filter(|other| {
            settings::configured_destination(settings, *other)
                .is_some_and(|path| same_directory(&destination, path))
        })
        .map(Harness::label)
        .collect();
    if !shared.is_empty() {
        warnings.push(format!(
            "This destination is also configured for {}.",
            shared.join(", ")
        ));
    }
    if harness == Harness::OpenCode {
        warnings.push("OpenCode also reads Claude Code and shared agent skill directories; this tab manages only its configured destination.".into());
    }
    let revision = format!(
        "scan-{:016x}",
        NEXT_REVISION.fetch_add(1, Ordering::Relaxed)
    );
    Ok(ScanResponse {
        revision,
        harness,
        source_path: source,
        destination_path: destination,
        skills,
        warnings,
        resolved_source_path,
        resolved_destination_path,
    })
}

fn read_source(path: &Path) -> Result<BTreeMap<String, SkillInfo>, String> {
    read_folders(path, true)
}

fn read_installed(path: &Path) -> Result<BTreeMap<String, SkillInfo>, String> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => {
            return Err(format!(
                "Could not inspect destination {}: {error}",
                path.display()
            ))
        }
        Ok(_) => {}
    }
    read_folders(path, false)
}

fn read_folders(parent: &Path, source: bool) -> Result<BTreeMap<String, SkillInfo>, String> {
    let entries = fs::read_dir(parent)
        .map_err(|error| format!("Could not read {}: {error}", parent.display()))?;
    let mut folders = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("Could not enumerate {}: {error}", parent.display()))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;
        if metadata.is_dir() || metadata.file_type().is_symlink() && path.is_dir() {
            folders.push(path);
        }
    }
    folders.sort();

    let mut found: BTreeMap<String, SkillInfo> = BTreeMap::new();
    for path in folders {
        let folder_name = path.file_name().unwrap().to_string_lossy().into_owned();
        let key = identity_key(&folder_name);
        let skill_file = path.join("SKILL.md");
        let skill_exists = fs::symlink_metadata(&skill_file).is_ok();
        let skill_is_file = skill_file.is_file();
        if !source && (!skill_exists || !skill_is_file) {
            continue;
        }
        let mut skill = SkillInfo {
            path,
            ..SkillInfo::default()
        };
        match fs::read_to_string(&skill_file) {
            Ok(content) => match parse_metadata(&content) {
                Ok((name, description)) => {
                    skill.name = name;
                    skill.description = description;
                    if skill.name.as_deref().is_none_or(str::is_empty) {
                        skill.warnings.push("Metadata name is missing; the folder name is used as the display name.".into());
                    }
                    if skill.description.as_deref().is_none_or(str::is_empty) {
                        skill
                            .warnings
                            .push("Metadata description is missing.".into());
                    }
                }
                Err(error) => {
                    skill.warnings.push(format!("Metadata could not be read; the folder name is used as the display name: {error}"));
                }
            },
            Err(error) if source => {
                skill.metadata_error = Some(if error.kind() == io::ErrorKind::NotFound {
                    "SKILL.md is missing.".into()
                } else {
                    format!("SKILL.md cannot be read: {error}")
                });
                skill
                    .warnings
                    .push("The folder name is used as the display name.".into());
            }
            Err(error) => {
                skill.warnings.push(format!("Installed SKILL.md cannot be read; the identified folder remains visible: {error}"));
            }
        }
        if let Some(existing) = found.get_mut(&key) {
            existing.ambiguous = true;
        } else {
            found.insert(key, skill);
        }
    }
    Ok(found)
}

fn parse_metadata(content: &str) -> Result<(Option<String>, Option<String>), String> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err("SKILL.md has no YAML frontmatter".into());
    }
    let mut yaml_lines = Vec::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        yaml_lines.push(line);
    }
    if !closed {
        return Err("YAML frontmatter is not closed".into());
    }
    let yaml = yaml_lines.join("\n");
    let metadata: Metadata = serde_saphyr::from_str(&yaml).map_err(|error| error.to_string())?;
    Ok((
        metadata
            .name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        metadata
            .description
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    ))
}

fn compare_trees(left: &TreeScan, right: &TreeScan) -> (bool, Vec<Difference>) {
    let keys: BTreeSet<_> = left
        .entries
        .keys()
        .chain(right.entries.keys())
        .cloned()
        .collect();
    let mut differences = Vec::new();
    for path in keys {
        let kind = match (left.entries.get(&path), right.entries.get(&path)) {
            (None, Some(_)) => Some("removed"),
            (Some(_), None) => Some("added"),
            (Some(Entry::Directory), Some(Entry::Directory)) => None,
            (
                Some(Entry::Link {
                    target_is_dir: a, ..
                }),
                Some(Entry::Link {
                    target_is_dir: b, ..
                }),
            ) if a != b => Some("type_changed"),
            (Some(Entry::File(a)), Some(Entry::File(b))) if a == b => None,
            (
                Some(Entry::Link {
                    target_is_dir: a,
                    content: ah,
                }),
                Some(Entry::Link {
                    target_is_dir: b,
                    content: bh,
                }),
            ) if a == b && ah == bh => None,
            (Some(a), Some(b)) if std::mem::discriminant(a) != std::mem::discriminant(b) => {
                Some("type_changed")
            }
            (Some(_), Some(_)) => Some("changed"),
            (None, None) => None,
        };
        if let Some(kind) = kind {
            differences.push(Difference {
                path,
                kind: kind.into(),
            });
        }
    }
    let equal = differences.is_empty();
    (equal, differences)
}

fn inventory(root: &Path) -> Result<TreeScan, String> {
    let root_metadata = fs::symlink_metadata(root)
        .map_err(|error| format!("Could not inspect {}: {error}", root.display()))?;
    let root_link = is_link_or_reparse(root, &root_metadata)?;
    let canonical = fs::canonicalize(root)
        .map_err(|error| format!("Could not resolve {}: {error}", root.display()))?;
    let mut result = TreeScan::default();
    let mut active = BTreeSet::new();
    let root_key = path_key(&canonical);
    active.insert(root_key.clone());
    if root_link {
        result.links.push(LinkWarning {
            path: ".".into(),
            target: canonical.display().to_string(),
        });
    }
    inventory_directory(&canonical, Path::new(""), &mut active, &mut result)?;
    Ok(result)
}

fn tree_fingerprint(tree: &TreeScan) -> String {
    let mut hash = Sha256::new();
    hash.update(format!("{:?}", tree.entries).as_bytes());
    hash.update(format!("{:?}", tree.links).as_bytes());
    format!("{:x}", hash.finalize())
}

pub(crate) fn fingerprint(path: &Path) -> Result<String, String> {
    inventory(path).map(|tree| tree_fingerprint(&tree))
}

pub(crate) fn content_fingerprint(path: &Path) -> Result<String, String> {
    let tree = inventory(path)?;
    let mut normalized = BTreeMap::new();
    for (path, entry) in tree.entries {
        let entry = match entry {
            Entry::Link {
                target_is_dir: true,
                ..
            } => Entry::Directory,
            Entry::Link {
                content: Some(hash),
                ..
            } => Entry::File(hash),
            other => other,
        };
        normalized.insert(path, entry);
    }
    let mut hash = Sha256::new();
    hash.update(format!("{:?}", normalized).as_bytes());
    Ok(format!("{:x}", hash.finalize()))
}

pub(crate) fn paths_overlap(left: &Path, right: &Path) -> bool {
    let left = path_key(&resolve_path(left).unwrap_or_else(|_| absolute_path(left)));
    let right = path_key(&resolve_path(right).unwrap_or_else(|_| absolute_path(right)));
    key_contains(&left, &right) || key_contains(&right, &left)
}

pub(crate) fn path_is_within(path: &Path, parent: &Path) -> bool {
    let path = path_key(&resolve_path(path).unwrap_or_else(|_| absolute_path(path)));
    let parent = path_key(&resolve_path(parent).unwrap_or_else(|_| absolute_path(parent)));
    key_contains(&path, &parent)
}

fn key_contains(path: &str, parent: &str) -> bool {
    Path::new(path).starts_with(Path::new(parent))
}

pub(crate) fn resolved_path(path: &Path) -> Result<PathBuf, String> {
    resolve_path(path)
}

pub(crate) fn path_key_for_ops(path: &Path) -> String {
    path_key(path)
}

pub(crate) fn is_link_or_reparse_path(path: &Path) -> Result<bool, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;
    is_link_or_reparse(path, &metadata)
}

fn resolve_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = absolute_path(path);
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    let mut candidate = normalized.clone();
    let mut missing = Vec::new();
    let base = loop {
        match fs::canonicalize(&candidate) {
            Ok(path) => break path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let Some(name) = candidate.file_name().map(|name| name.to_os_string()) else {
                    return Err(format!(
                        "Could not resolve configured path {}: {error}",
                        path.display()
                    ));
                };
                missing.push(name);
                if !candidate.pop() {
                    return Err(format!(
                        "Could not resolve configured path {}: {error}",
                        path.display()
                    ));
                }
            }
            Err(error) => {
                return Err(format!(
                    "Could not resolve configured path {}: {error}",
                    path.display()
                ))
            }
        }
    };
    Ok(missing
        .into_iter()
        .rev()
        .fold(base, |path, part| path.join(part)))
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    }
}

pub(crate) fn copy_materialized(
    source: &Path,
    destination: &Path,
    forbidden: &[PathBuf],
) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Could not inspect source {}: {error}", source.display()))?;
    let real = if is_link_or_reparse(source, &metadata)? {
        fs::canonicalize(source).map_err(|error| {
            format!(
                "Could not resolve source link {}: {error}",
                source.display()
            )
        })?
    } else {
        fs::canonicalize(source)
            .map_err(|error| format!("Could not resolve source {}: {error}", source.display()))?
    };
    reject_forbidden(&real, forbidden)?;
    if !fs::metadata(&real)
        .map_err(|error| format!("Could not inspect source {}: {error}", real.display()))?
        .is_dir()
    {
        return Err(format!(
            "Skill source is not a directory: {}",
            source.display()
        ));
    }
    let mut active = BTreeSet::from([path_key(&real)]);
    copy_directory_contents(&real, destination, &mut active, forbidden)
}

fn reject_forbidden(path: &Path, forbidden: &[PathBuf]) -> Result<(), String> {
    for root in forbidden {
        let resolved = fs::canonicalize(root).unwrap_or_else(|_| absolute_path(root));
        if paths_overlap(path, &resolved) {
            return Err(format!(
                "Link target {} overlaps protected path {}",
                path.display(),
                resolved.display()
            ));
        }
    }
    Ok(())
}

fn copy_directory_contents(
    real: &Path,
    output: &Path,
    active: &mut BTreeSet<String>,
    forbidden: &[PathBuf],
) -> Result<(), String> {
    let mut entries = fs::read_dir(real)
        .map_err(|error| format!("Could not read {}: {error}", real.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not enumerate {}: {error}", real.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source = entry.path();
        let destination = output.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source)
            .map_err(|error| format!("Could not inspect {}: {error}", source.display()))?;
        let is_link = is_link_or_reparse(&source, &metadata)?;
        let resolved = if is_link || metadata.is_dir() {
            Some(
                fs::canonicalize(&source)
                    .map_err(|error| format!("Could not resolve {}: {error}", source.display()))?,
            )
        } else {
            None
        };
        if let Some(real_child) = resolved {
            reject_forbidden(&real_child, forbidden)?;
            let target_metadata = fs::metadata(&real_child)
                .map_err(|error| format!("Could not inspect {}: {error}", real_child.display()))?;
            if target_metadata.is_dir() {
                let key = path_key(&real_child);
                if !active.insert(key.clone()) {
                    return Err(format!("Link cycle detected at {}", source.display()));
                }
                fs::create_dir(&destination).map_err(|error| {
                    format!("Could not stage {}: {error}", destination.display())
                })?;
                let copied = copy_directory_contents(&real_child, &destination, active, forbidden);
                active.remove(&key);
                copied?;
            } else if target_metadata.is_file() {
                fs::copy(&real_child, &destination).map_err(|error| {
                    format!("Could not stage {}: {error}", destination.display())
                })?;
            } else {
                return Err(format!(
                    "Unsupported filesystem entry at {}",
                    source.display()
                ));
            }
        } else if metadata.is_file() {
            fs::copy(&source, &destination)
                .map_err(|error| format!("Could not stage {}: {error}", destination.display()))?;
        } else {
            return Err(format!(
                "Unsupported filesystem entry at {}",
                source.display()
            ));
        }
    }
    Ok(())
}

fn inventory_directory(
    real: &Path,
    relative: &Path,
    active: &mut BTreeSet<String>,
    result: &mut TreeScan,
) -> Result<(), String> {
    let mut entries = fs::read_dir(real)
        .map_err(|error| format!("Could not read {}: {error}", real.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Could not enumerate {}: {error}", real.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let child_relative = relative.join(name);
        let display = display_path(&child_relative);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("Could not inspect {display}: {error}"))?;
        if is_link_or_reparse(&path, &metadata)? {
            let target = fs::canonicalize(&path)
                .map_err(|error| format!("Could not resolve link {display}: {error}"))?;
            let target_metadata = fs::metadata(&target).map_err(|error| {
                format!(
                    "Could not inspect link target {}: {error}",
                    target.display()
                )
            })?;
            result.links.push(LinkWarning {
                path: display.clone(),
                target: target.display().to_string(),
            });
            if target_metadata.is_dir() {
                result.entries.insert(
                    display.clone(),
                    Entry::Link {
                        target_is_dir: true,
                        content: None,
                    },
                );
                let key = path_key(&target);
                if !active.insert(key.clone()) {
                    return Err(format!(
                        "Link cycle detected at {display} targeting {}",
                        target.display()
                    ));
                }
                let walk = inventory_directory(&target, &child_relative, active, result);
                active.remove(&key);
                walk?;
            } else if target_metadata.is_file() {
                result.entries.insert(
                    display,
                    Entry::Link {
                        target_is_dir: false,
                        content: Some(hash_file(&target).map_err(|error| {
                            format!("Could not read link target {}: {error}", target.display())
                        })?),
                    },
                );
            } else {
                return Err(format!(
                    "Unsupported link target type at {display}: {}",
                    target.display()
                ));
            }
        } else if metadata.is_dir() {
            result.entries.insert(display.clone(), Entry::Directory);
            let canonical = fs::canonicalize(&path)
                .map_err(|error| format!("Could not resolve {display}: {error}"))?;
            let key = path_key(&canonical);
            if !active.insert(key.clone()) {
                return Err(format!("Directory cycle detected at {display}"));
            }
            let walk = inventory_directory(&canonical, &child_relative, active, result);
            active.remove(&key);
            walk?;
        } else if metadata.is_file() {
            result.entries.insert(
                display.clone(),
                Entry::File(
                    hash_file(&path)
                        .map_err(|error| format!("Could not read {display}: {error}"))?,
                ),
            );
        } else {
            return Err(format!("Unsupported filesystem entry at {display}"));
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> io::Result<[u8; 32]> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().into())
}

#[cfg(not(windows))]
fn is_link_or_reparse(_path: &Path, metadata: &fs::Metadata) -> Result<bool, String> {
    Ok(metadata.file_type().is_symlink())
}

#[cfg(windows)]
fn is_link_or_reparse(path: &Path, metadata: &fs::Metadata) -> Result<bool, String> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FileAttributeTagInfo, GetFileInformationByHandleEx,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE,
        },
    };

    use std::os::windows::fs::MetadataExt;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0 {
        return Ok(false);
    }
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // Open the directory entry itself so the returned tag identifies the reparse point.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            3,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(format!(
            "Could not inspect Windows reparse point {}: {}",
            path.display(),
            io::Error::last_os_error()
        ));
    }
    let mut info = FILE_ATTRIBUTE_TAG_INFO {
        FileAttributes: 0,
        ReparseTag: 0,
    };
    let ok = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of_val(&info) as u32,
        )
    };
    let error = (ok == 0).then(io::Error::last_os_error);
    unsafe { CloseHandle(handle) };
    if let Some(error) = error {
        return Err(format!(
            "Could not inspect Windows reparse tag for {}: {}",
            path.display(),
            error
        ));
    }
    match info.ReparseTag {
        0xA000000C | 0xA0000003 => Ok(true),
        tag => Err(format!(
            "Unsupported Windows reparse point at {} (tag 0x{tag:08x})",
            path.display()
        )),
    }
}

fn identity_key(folder: &str) -> String {
    if cfg!(windows) {
        folder.to_lowercase()
    } else {
        folder.to_string()
    }
}

fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value.into_owned()
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn same_directory(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => path_key(&left) == path_key(&right),
        _ => path_key(left) == path_key(right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};

    fn file(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut output = fs::File::create(path).unwrap();
        output.write_all(bytes).unwrap();
    }

    fn skill(parent: &Path, name: &str, body: &str) {
        file(
            &parent.join(name).join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\n{body}").as_bytes(),
        );
    }

    #[cfg(unix)]
    fn link_dir(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(windows)]
    fn link_dir(target: &Path, link: &Path) {
        let command = format!("mklink /J {} {}", link.display(), target.display());
        let result = std::process::Command::new("cmd")
            .args(["/C", &command])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "junction creation failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn complete_tree_comparison_covers_contents_types_hidden_and_empty_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(source.join("empty")).unwrap();
        fs::create_dir_all(destination.join("empty")).unwrap();
        file(&source.join(".hidden/data"), b"same");
        file(&destination.join(".hidden/data"), b"same");
        file(&source.join("nested.txt"), b"one");
        file(&destination.join("nested.txt"), b"two");
        file(&source.join("type"), b"file");
        fs::create_dir_all(destination.join("type")).unwrap();
        file(&destination.join("obsolete"), b"old");
        let source_tree = inventory(&source).unwrap();
        let destination_tree = inventory(&destination).unwrap();
        let (_, differences) = compare_trees(&source_tree, &destination_tree);
        assert!(differences
            .iter()
            .any(|d| d.path == "nested.txt" && d.kind == "changed"));
        assert!(differences
            .iter()
            .any(|d| d.path == "type" && d.kind == "type_changed"));
        assert!(differences
            .iter()
            .any(|d| d.path == "obsolete" && d.kind == "removed"));
        assert!(!differences.iter().any(|d| d.path == "empty"));
    }

    #[test]
    fn metadata_failures_fall_back_and_invalid_sources_are_reported() {
        assert_eq!(
            parse_metadata("---\nname: a\ndescription: b\n---\nbody").unwrap(),
            (Some("a".into()), Some("b".into()))
        );
        assert!(parse_metadata("---\n: bad\n---").is_err());
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("NoSkill")).unwrap();
        let skills = read_source(temp.path()).unwrap();
        assert!(skills[&identity_key("NoSkill")].metadata_error.is_some());

        let malformed = temp.path().join("Malformed");
        file(
            &malformed.join("SKILL.md"),
            b"---\nname: ok\nbody without a closing marker",
        );
        let skills = read_source(temp.path()).unwrap();
        assert!(skills[&identity_key("Malformed")].metadata_error.is_none());
        assert!(skills[&identity_key("Malformed")].name.is_none());
        assert!(!parse_metadata("---\nname: open").is_ok());
    }

    #[test]
    fn invalid_utf8_skill_file_is_invalid_source() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("source");
        fs::create_dir_all(parent.join("BadUtf8")).unwrap();
        fs::write(parent.join("BadUtf8").join("SKILL.md"), [0xff, 0xfe]).unwrap();
        let skills = read_source(&parent).unwrap();
        assert!(skills[&identity_key("BadUtf8")].metadata_error.is_some());
    }

    #[test]
    fn scan_reports_all_four_comparison_statuses() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&destination).unwrap();
        skill(&source, "Missing", "source only");
        skill(&source, "Identical", "same");
        skill(&destination, "Identical", "same");
        skill(&source, "Different", "source");
        skill(&destination, "Different", "target");
        skill(&destination, "Installed", "installed only");
        let settings = Settings {
            source: Some(source),
            destinations: [(Harness::Codex, destination)].into_iter().collect(),
            ..Settings::default()
        };
        let response = scan(&settings, Harness::Codex).unwrap();
        let statuses: BTreeMap<_, _> = response
            .skills
            .iter()
            .map(|row| (row.folder_name.as_str(), row.status.as_str()))
            .collect();
        assert_eq!(statuses["Missing"], "missing");
        assert_eq!(statuses["Identical"], "identical");
        assert_eq!(statuses["Different"], "different");
        assert_eq!(statuses["Installed"], "installed_only");
    }

    #[test]
    fn destination_only_link_warnings_and_shared_destinations_are_visible() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let target = temp.path().join("external");
        fs::create_dir_all(&destination).unwrap();
        file(&target.join("data.txt"), b"unchanged");
        skill(&destination, "Installed", "body");
        link_dir(&target, &destination.join("Installed").join("resources"));
        let mut settings = Settings::default();
        settings
            .destinations
            .insert(Harness::OpenCode, destination.clone());
        settings.destinations.insert(Harness::Claude, destination);
        let response = scan(&settings, Harness::OpenCode).unwrap();
        assert!(response
            .warnings
            .iter()
            .any(|warning| warning.contains("Claude Code")));
        assert_eq!(response.skills[0].status, "installed_only");
        assert!(response.skills[0]
            .link_warnings
            .iter()
            .any(|warning| warning.path == "resources"));
        assert_eq!(fs::read(target.join("data.txt")).unwrap(), b"unchanged");
    }

    #[test]
    fn root_links_are_reported_and_materialized_for_comparison() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let linked = temp.path().join("linked");
        fs::create_dir_all(&target).unwrap();
        file(&target.join("data.txt"), b"same");
        link_dir(&target, &linked);
        let tree = inventory(&linked).unwrap();
        assert!(tree.links.iter().any(|warning| warning.path == "."));
        assert!(tree.entries.contains_key("data.txt"));
        let plain = inventory(&target).unwrap();
        assert!(compare_trees(&tree, &plain).0);

        let output = temp.path().join("output");
        fs::create_dir(&output).unwrap();
        copy_materialized(&linked, &output, &[]).unwrap();
        assert_eq!(fs::read(output.join("data.txt")).unwrap(), b"same");
        assert!(fs::symlink_metadata(output.join("data.txt"))
            .unwrap()
            .is_file());
    }

    #[test]
    fn materialized_external_directory_link_keeps_its_target_unchanged() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("external");
        let output = temp.path().join("output");
        fs::create_dir_all(source.join("Skill")).unwrap();
        file(&target.join("nested/data.txt"), b"external contents");
        link_dir(&target, &source.join("Skill").join("assets"));
        fs::create_dir(&output).unwrap();

        copy_materialized(&source.join("Skill"), &output, &[]).unwrap();
        assert_eq!(
            fs::read(output.join("assets/nested/data.txt")).unwrap(),
            b"external contents"
        );
        let copied = output.join("assets");
        assert!(!is_link_or_reparse(&copied, &fs::symlink_metadata(&copied).unwrap()).unwrap());
        assert_eq!(
            fs::read(target.join("nested/data.txt")).unwrap(),
            b"external contents"
        );
    }

    #[test]
    fn repeated_non_cyclic_links_to_one_target_are_allowed() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let target = temp.path().join("target");
        fs::create_dir_all(&root).unwrap();
        file(&target.join("data.txt"), b"same");
        link_dir(&target, &root.join("first"));
        link_dir(&target, &root.join("second"));
        let tree = inventory(&root).unwrap();
        assert!(tree.entries.contains_key("first/data.txt"));
        assert!(tree.entries.contains_key("second/data.txt"));
    }

    #[test]
    fn missing_and_broken_inputs_report_errors() {
        let temp = tempfile::tempdir().unwrap();
        assert!(inventory(&temp.path().join("missing")).is_err());
        let root = temp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let target = temp.path().join("target");
        fs::create_dir_all(&target).unwrap();
        let link = root.join("broken");
        link_dir(&target, &link);
        fs::remove_dir_all(&target).unwrap();
        assert!(inventory(&root).unwrap_err().contains("link"));
    }

    #[test]
    fn installed_folder_with_directory_skill_md_is_not_identified() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        fs::create_dir_all(destination.join("Unknown").join("SKILL.md")).unwrap();
        assert!(read_installed(&destination).unwrap().is_empty());
    }

    #[test]
    fn cycles_fail_instead_of_skipping_links() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        link_dir(&root, &root.join("loop"));
        assert!(inventory(&root).unwrap_err().contains("cycle"));
    }

    #[test]
    fn case_identity_collides_on_windows_and_is_literal_elsewhere() {
        if cfg!(windows) {
            assert_eq!(identity_key("Skill"), identity_key("skill"));
        } else {
            assert_ne!(identity_key("Skill"), identity_key("skill"));
        }
    }

    #[test]
    fn overlap_resolves_missing_destination_below_a_linked_parent_and_volume_root() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let source = target.join("source");
        fs::create_dir_all(&source).unwrap();
        let alias = temp.path().join("alias");
        link_dir(&target, &alias);
        let missing_destination = alias.join("source").join("new").join("skills");
        assert!(paths_overlap(&source, &missing_destination));
        assert!(paths_overlap(&source, temp.path()));
    }
}
