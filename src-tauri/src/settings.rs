use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

use crate::manager::{Harness, BUILT_INS};

#[derive(Clone, Debug)]
struct ConfigurationRoots {
    codex: Option<PathBuf>,
    claude: Option<PathBuf>,
    antigravity: Option<PathBuf>,
    open_code: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessDescriptor {
    pub id: Harness,
    pub label: String,
    pub built_in: bool,
    pub visible: bool,
    pub destination_available: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub source: Option<PathBuf>,
    pub destinations: BTreeMap<Harness, PathBuf>,
    pub custom_harnesses: BTreeMap<Harness, String>,
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source: None,
            destinations: BTreeMap::new(),
            custom_harnesses: BTreeMap::new(),
            theme: Theme::Dark,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponse {
    pub settings: Settings,
    pub settings_file: PathBuf,
    pub error: Option<String>,
    pub harnesses: Vec<HarnessDescriptor>,
}

pub fn defaults() -> Settings {
    defaults_with_roots(&configuration_roots())
}

fn defaults_with_roots(roots: &ConfigurationRoots) -> Settings {
    let mut settings = Settings::default();
    for harness in BUILT_INS {
        if let Some(path) = default_destination(harness, roots) {
            settings.destinations.insert(harness.to_string(), path);
        }
    }
    settings
}

fn configuration_roots() -> ConfigurationRoots {
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    resolve_configuration_roots(
        home,
        std::env::var_os("CODEX_HOME").map(PathBuf::from),
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
    )
}

fn resolve_configuration_roots(
    home: Option<PathBuf>,
    codex_home: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
) -> ConfigurationRoots {
    ConfigurationRoots {
        codex: codex_home.or_else(|| home.as_ref().map(|path| path.join(".codex"))),
        claude: home.as_ref().map(|path| path.join(".claude")),
        antigravity: home
            .as_ref()
            .map(|path| path.join(".gemini").join("config")),
        open_code: xdg_config_home
            .or_else(|| home.map(|path| path.join(".config")))
            .map(|path| path.join("opencode")),
    }
}

fn configuration_root<'a>(harness: &str, roots: &'a ConfigurationRoots) -> Option<&'a Path> {
    match harness {
        "codex" => roots.codex.as_deref(),
        "claude" => roots.claude.as_deref(),
        "antigravity" => roots.antigravity.as_deref(),
        "open_code" => roots.open_code.as_deref(),
        _ => None,
    }
}

fn default_destination(harness: &str, roots: &ConfigurationRoots) -> Option<PathBuf> {
    configuration_root(harness, roots).map(|path| path.join("skills"))
}

fn harness_descriptors(settings: &Settings, roots: &ConfigurationRoots) -> Vec<HarnessDescriptor> {
    let mut descriptors: Vec<_> = BUILT_INS
        .into_iter()
        .map(|id| {
            let default = default_destination(id, roots);
            let destination = settings.destinations.get(id);
            let destination_available = destination.is_some_and(|path| path.is_dir());
            let configured_override = destination
                .is_some_and(|path| default.as_ref().is_none_or(|default| path != default));
            let configured = configuration_root(id, roots).is_some_and(Path::is_dir);
            HarnessDescriptor {
                id: id.to_string(),
                label: builtin_label(id).unwrap_or(id).to_string(),
                built_in: true,
                visible: configured || (configured_override && destination_available),
                destination_available,
            }
        })
        .collect();
    descriptors.extend(settings.custom_harnesses.iter().map(|(id, name)| {
        HarnessDescriptor {
            id: id.clone(),
            label: name.clone(),
            built_in: false,
            visible: true,
            destination_available: settings
                .destinations
                .get(id)
                .is_some_and(|path| path.is_dir()),
        }
    }));
    descriptors
}

fn builtin_label(id: &str) -> Option<&'static str> {
    match id {
        "codex" => Some("Codex"),
        "claude" => Some("Claude Code"),
        "antigravity" => Some("Antigravity IDE"),
        "open_code" => Some("OpenCode"),
        _ => None,
    }
}

pub fn harness_label<'a>(settings: &'a Settings, id: &str) -> Option<&'a str> {
    if BUILT_INS.contains(&id) {
        builtin_label(id)
    } else {
        settings.custom_harnesses.get(id).map(String::as_str)
    }
}

fn settings_file(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("settings.json"))
        .map_err(|error| format!("Could not locate the app settings folder: {error}"))
}

pub fn load(app: &AppHandle) -> SettingsResponse {
    let file = match settings_file(app) {
        Ok(file) => file,
        Err(error) => {
            let settings = defaults();
            return SettingsResponse {
                harnesses: harness_descriptors(&settings, &configuration_roots()),
                settings,
                settings_file: PathBuf::new(),
                error: Some(error),
            };
        }
    };
    let (settings, error) = load_file(&file);
    let harnesses = harness_descriptors(&settings, &configuration_roots());
    SettingsResponse {
        settings,
        settings_file: file,
        error,
        harnesses,
    }
}

fn load_file(file: &Path) -> (Settings, Option<String>) {
    match fs::read(file) {
        Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
            Ok(mut settings) => {
                if let Err(error) = validate_registry(&settings) {
                    return (defaults(), Some(format!(
                        "Settings at {} are invalid: {error}. Correct the file before changing settings.", file.display()
                    )));
                }
                for (harness, path) in defaults().destinations {
                    settings.destinations.entry(harness).or_insert(path);
                }
                (settings, None)
            }
            Err(error) => (defaults(), Some(format!(
                "Settings at {} are invalid: {error}. Correct the file before changing settings.", file.display()
            ))),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (defaults(), None),
        Err(error) => (defaults(), Some(format!("Could not read {}: {error}", file.display()))),
    }
}

pub fn save(app: &AppHandle, settings: Settings) -> Result<(), String> {
    let file = settings_file(app)?;
    save_file(&file, settings)
}

fn save_file(file: &Path, settings: Settings) -> Result<(), String> {
    let previous = match fs::read(file) {
        Ok(bytes) => {
            let previous = serde_json::from_slice::<Settings>(&bytes).map_err(|error| {
                format!(
                    "Settings at {} are invalid and were not overwritten: {error}",
                    file.display()
                )
            })?;
            validate_registry(&previous).map_err(|error| {
                format!(
                    "Settings at {} are invalid and were not overwritten: {error}",
                    file.display()
                )
            })?;
            Some(previous)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(format!("Could not read {}: {error}", file.display())),
    };
    validate_registry(&settings)?;
    if previous
        .as_ref()
        .is_none_or(|old| old.source != settings.source)
    {
        validate_source(&settings)?;
    }
    validate_custom_destinations(&settings, previous.as_ref())?;

    let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?;
    let parent = file
        .parent()
        .ok_or_else(|| "Settings path has no parent folder".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Could not create settings folder {}: {error}",
            parent.display()
        )
    })?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!("settings-{now}-{}.tmp", std::process::id()));
    let write_result = (|| {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        output.write_all(&bytes)?;
        output.sync_all()?;
        fs::rename(&temporary, file)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result.map_err(|error| format!("Could not save {}: {error}", file.display()))
}

fn validate_registry(settings: &Settings) -> Result<(), String> {
    let built_ins: BTreeSet<&str> = BUILT_INS.into_iter().collect();
    for id in settings.destinations.keys() {
        if !built_ins.contains(id.as_str()) && !settings.custom_harnesses.contains_key(id) {
            return Err(format!("Unknown harness identity '{id}'"));
        }
    }
    for (id, name) in &settings.custom_harnesses {
        if !valid_custom_id(id) || built_ins.contains(id.as_str()) {
            return Err(format!("Malformed custom harness identity '{id}'"));
        }
        if name.trim().is_empty() {
            return Err(format!("Custom harness '{id}' needs a nonempty name"));
        }
        if !settings.destinations.contains_key(id) {
            return Err(format!("Custom harness '{id}' has no destination"));
        }
    }
    Ok(())
}

fn valid_custom_id(id: &str) -> bool {
    let Some(uuid) = id.strip_prefix("custom-") else {
        return false;
    };
    uuid.len() == 36
        && uuid.bytes().enumerate().all(|(i, byte)| {
            if [8, 13, 18, 23].contains(&i) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn validate_custom_destinations(
    settings: &Settings,
    previous: Option<&Settings>,
) -> Result<(), String> {
    for id in settings.custom_harnesses.keys() {
        let destination = &settings.destinations[id];
        let changed = previous.and_then(|old| old.destinations.get(id)) != Some(destination);
        if changed {
            fs::read_dir(destination).map_err(|error| {
                format!(
                    "Custom harness destination is not an accessible directory: {}: {error}",
                    destination.display()
                )
            })?;
        }
    }
    Ok(())
}

fn validate_source(settings: &Settings) -> Result<(), String> {
    if let Some(source) = &settings.source {
        fs::read_dir(source).map_err(|error| {
            format!(
                "Source folder is not accessible: {}: {error}",
                source.display()
            )
        })?;
    }
    Ok(())
}

pub fn configured_destination<'a>(settings: &'a Settings, harness: &str) -> Option<&'a Path> {
    harness_label(settings, harness)?;
    settings.destinations.get(harness).map(PathBuf::as_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CUSTOM: &str = "custom-550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn old_settings_load_with_empty_custom_registry_and_builtin_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("settings.json");
        let source = directory.path().join("source");
        let override_path = directory.path().join("codex-override");
        let mut old = serde_json::to_value(Settings {
            source: Some(source.clone()),
            destinations: [("codex".into(), override_path.clone())].into(),
            theme: Theme::Light,
            ..Settings::default()
        })
        .unwrap();
        old.as_object_mut().unwrap().remove("custom_harnesses");
        let original = serde_json::to_vec(&old).unwrap();
        fs::write(&file, &original).unwrap();
        let (settings, error) = load_file(&file);
        assert!(error.is_none());
        assert_eq!(settings.custom_harnesses.len(), 0);
        assert_eq!(settings.destinations.len(), 4);
        assert_eq!(settings.destinations["codex"], override_path);
        assert_eq!(settings.source, Some(source));
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(fs::read(&file).unwrap(), original);
    }

    #[test]
    fn defaults_resolve_all_harnesses_without_creating_directories() {
        let home = tempfile::tempdir().unwrap();
        let roots = resolve_configuration_roots(Some(home.path().to_path_buf()), None, None);
        let settings = defaults_with_roots(&roots);
        assert_eq!(settings.destinations.len(), 4);
        assert_eq!(settings.theme, Theme::Dark);
        assert!(settings
            .destinations
            .values()
            .all(|destination| !destination.exists()));
        assert!(home.path().read_dir().unwrap().next().is_none());
    }

    #[test]
    fn configuration_roots_follow_runtime_environment_values() {
        let home = PathBuf::from("home-root");
        let codex_home = PathBuf::from("codex-root");
        let xdg_home = PathBuf::from("xdg-root");
        let roots = resolve_configuration_roots(
            Some(home.clone()),
            Some(codex_home.clone()),
            Some(xdg_home.clone()),
        );
        assert_eq!(roots.codex, Some(codex_home));
        assert_eq!(roots.claude, Some(home.join(".claude")));
        assert_eq!(roots.antigravity, Some(home.join(".gemini").join("config")));
        assert_eq!(roots.open_code, Some(xdg_home.join("opencode")));
    }

    #[test]
    fn built_in_visibility_uses_existing_configuration_or_directory_override() {
        let root = tempfile::tempdir().unwrap();
        let codex_config = root.path().join(".codex");
        let override_destination = root.path().join("override");
        fs::create_dir(&codex_config).unwrap();
        fs::create_dir(&override_destination).unwrap();
        let roots = resolve_configuration_roots(Some(root.path().into()), None, None);
        let mut settings = Settings::default();
        settings
            .destinations
            .insert("codex".into(), codex_config.join("skills"));
        settings
            .destinations
            .insert("claude".into(), override_destination.clone());
        let descriptors = harness_descriptors(&settings, &roots);
        assert!(descriptors[0].visible);
        assert!(!descriptors[0].destination_available);
        assert!(descriptors[1].visible && descriptors[1].destination_available);
        assert!(!descriptors[2].visible);
        assert!(!descriptors[3].visible);
        assert_eq!(fs::read_dir(&codex_config).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&override_destination).unwrap().count(), 0);
    }

    #[test]
    fn override_is_visible_without_a_resolved_default_root() {
        let root = tempfile::tempdir().unwrap();
        let override_destination = root.path().join("skills");
        fs::create_dir(&override_destination).unwrap();
        let settings = Settings {
            destinations: [("codex".into(), override_destination)].into(),
            ..Settings::default()
        };
        let descriptors =
            harness_descriptors(&settings, &resolve_configuration_roots(None, None, None));
        assert!(descriptors[0].visible);
        assert!(descriptors[0].destination_available);
    }

    #[test]
    fn descriptors_keep_missing_custom_and_filter_builtins() {
        let root = tempfile::tempdir().unwrap();
        let codex_config = root.path().join(".codex");
        fs::create_dir(&codex_config).unwrap();
        let missing = root.path().join("missing-custom");
        let mut settings = Settings::default();
        settings
            .destinations
            .insert("codex".into(), codex_config.join("skills"));
        settings.destinations.insert(CUSTOM.into(), missing.clone());
        settings
            .custom_harnesses
            .insert(CUSTOM.into(), "Research".into());
        let descriptors = harness_descriptors(
            &settings,
            &resolve_configuration_roots(Some(root.path().into()), None, None),
        );
        assert!(descriptors[0].visible);
        assert_eq!(descriptors.last().unwrap().id, CUSTOM);
        assert!(descriptors.last().unwrap().visible);
        assert!(!descriptors.last().unwrap().destination_available);
        assert!(!missing.exists());
    }

    #[test]
    fn registry_rejects_unknown_malformed_and_incomplete_entries() {
        let mut settings = Settings::default();
        settings
            .destinations
            .insert("rogue".into(), PathBuf::from("x"));
        assert!(validate_registry(&settings)
            .unwrap_err()
            .contains("Unknown"));
        settings.destinations.clear();
        settings
            .custom_harnesses
            .insert("custom-bad".into(), "Name".into());
        assert!(validate_registry(&settings)
            .unwrap_err()
            .contains("Malformed"));
        settings.custom_harnesses.clear();
        settings.custom_harnesses.insert(CUSTOM.into(), " ".into());
        assert!(validate_registry(&settings)
            .unwrap_err()
            .contains("nonempty"));
        settings
            .custom_harnesses
            .insert(CUSTOM.into(), "Name".into());
        assert!(validate_registry(&settings)
            .unwrap_err()
            .contains("no destination"));
    }

    #[test]
    fn custom_destination_is_checked_only_when_added_or_changed() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("skills");
        fs::create_dir(&destination).unwrap();
        let mut settings = defaults();
        settings
            .destinations
            .insert(CUSTOM.into(), destination.clone());
        settings
            .custom_harnesses
            .insert(CUSTOM.into(), "Research".into());
        validate_custom_destinations(&settings, None).unwrap();
        let file = root.path().join("settings.json");
        save_file(&file, settings.clone()).unwrap();
        let previous = settings.clone();
        fs::remove_dir(&destination).unwrap();
        let mut renamed = settings;
        renamed
            .custom_harnesses
            .insert(CUSTOM.into(), "Renamed".into());
        validate_custom_destinations(&renamed, Some(&previous)).unwrap();
        assert!(validate_custom_destinations(&renamed, None).is_err());
        let original = fs::read(&file).unwrap();
        let mut invalid_path = previous.clone();
        invalid_path
            .destinations
            .insert(CUSTOM.into(), root.path().join("other-missing"));
        assert!(save_file(&file, invalid_path).is_err());
        assert_eq!(fs::read(&file).unwrap(), original);
        renamed.theme = Theme::Light;
        save_file(&file, renamed).unwrap();
        let (reloaded, error) = load_file(&file);
        assert!(error.is_none());
        assert_eq!(reloaded.custom_harnesses[CUSTOM], "Renamed");
        assert_eq!(reloaded.theme, Theme::Light);
    }

    #[test]
    fn custom_registration_rename_and_removal_persist_without_touching_folder() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("skills");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("keep.txt"), "untouched").unwrap();
        let file = root.path().join("settings.json");
        let mut settings = defaults();
        settings.destinations.insert(CUSTOM.into(), folder.clone());
        settings
            .custom_harnesses
            .insert(CUSTOM.into(), "Research".into());
        save_file(&file, settings.clone()).unwrap();
        settings
            .custom_harnesses
            .insert(CUSTOM.into(), "Renamed".into());
        save_file(&file, settings.clone()).unwrap();
        let (loaded, error) = load_file(&file);
        assert!(error.is_none());
        assert_eq!(loaded.custom_harnesses[CUSTOM], "Renamed");
        settings.custom_harnesses.remove(CUSTOM);
        settings.destinations.remove(CUSTOM);
        save_file(&file, settings).unwrap();
        let (loaded, error) = load_file(&file);
        assert!(error.is_none());
        assert!(!loaded.custom_harnesses.contains_key(CUSTOM));
        assert_eq!(
            fs::read_to_string(folder.join("keep.txt")).unwrap(),
            "untouched"
        );
    }

    #[test]
    fn semantic_malformed_settings_are_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("settings.json");
        fs::write(
            &file,
            r#"{"source":null,"destinations":{"rogue":"x"},"theme":"dark"}"#,
        )
        .unwrap();
        let original = fs::read(&file).unwrap();
        assert!(load_file(&file).1.unwrap().contains("Unknown harness"));
        assert!(save_file(&file, Settings::default())
            .unwrap_err()
            .contains("Unknown harness"));
        assert_eq!(fs::read(&file).unwrap(), original);
    }

    #[test]
    fn malformed_settings_are_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, "{broken").unwrap();
        let original = fs::read(&path).unwrap();
        assert!(save_file(&path, Settings::default()).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }

    #[test]
    fn settings_round_trip_and_unavailable_remembered_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        fs::create_dir(&source).unwrap();
        let file = directory.path().join("settings.json");
        let settings = Settings {
            source: Some(source.clone()),
            theme: Theme::Light,
            ..defaults()
        };
        save_file(&file, settings).unwrap();
        let (mut loaded, error) = load_file(&file);
        assert!(error.is_none());
        assert_eq!(loaded.theme, Theme::Light);
        fs::remove_dir(&source).unwrap();
        loaded.theme = Theme::Dark;
        save_file(&file, loaded).unwrap();
        let (loaded, error) = load_file(&file);
        assert!(error.is_none());
        assert_eq!(loaded.source, Some(source));
    }
}
