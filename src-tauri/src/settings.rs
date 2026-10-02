use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

use crate::manager::Harness;

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
    pub label: &'static str,
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
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source: None,
            destinations: BTreeMap::new(),
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
    for harness in Harness::ALL {
        if let Some(path) = default_destination(harness, roots) {
            settings.destinations.insert(harness, path);
        }
    }
    settings
}

fn configuration_roots() -> ConfigurationRoots {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from));
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

fn configuration_root(harness: Harness, roots: &ConfigurationRoots) -> Option<&Path> {
    match harness {
        Harness::Codex => roots.codex.as_deref(),
        Harness::Claude => roots.claude.as_deref(),
        Harness::Antigravity => roots.antigravity.as_deref(),
        Harness::OpenCode => roots.open_code.as_deref(),
    }
}

fn default_destination(harness: Harness, roots: &ConfigurationRoots) -> Option<PathBuf> {
    configuration_root(harness, roots).map(|path| path.join("skills"))
}

fn harness_descriptors(settings: &Settings, roots: &ConfigurationRoots) -> Vec<HarnessDescriptor> {
    Harness::ALL
        .into_iter()
        .map(|id| {
            let default = default_destination(id, roots);
            let destination = settings.destinations.get(&id);
            let destination_available = destination.is_some_and(|path| path.is_dir());
            let configured_override = destination
                .is_some_and(|path| default.as_ref().is_none_or(|default| path != default));
            let configured = configuration_root(id, roots).is_some_and(Path::is_dir);
            let visible = configured || (configured_override && destination_available);
            HarnessDescriptor {
                id,
                label: id.label(),
                built_in: true,
                visible,
                destination_available,
            }
        })
        .collect()
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
                let baseline = defaults();
                for (harness, path) in baseline.destinations {
                    settings.destinations.entry(harness).or_insert(path);
                }
                (settings, None)
            }
            Err(error) => (
                defaults(),
                Some(format!(
                    "Settings at {} are invalid: {error}. Correct the file before changing settings.",
                    file.display()
                )),
            ),
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
    let mut source_changed = true;
    match fs::read(file) {
        Ok(bytes) => {
            let previous = serde_json::from_slice::<Settings>(&bytes).map_err(|error| {
                format!(
                    "Settings at {} are invalid and were not overwritten: {error}",
                    file.display()
                )
            })?;
            source_changed = settings.source != previous.source;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not read {}: {error}", file.display())),
    }
    if source_changed {
        validate_source(&settings)?;
    }

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

pub fn configured_destination(settings: &Settings, harness: Harness) -> Option<&Path> {
    settings.destinations.get(&harness).map(PathBuf::as_path)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn built_in_visibility_uses_configuration_roots_and_directory_overrides() {
        let root = tempfile::tempdir().unwrap();
        let codex_config = root.path().join(".codex");
        let override_destination = root.path().join("custom-skills");
        fs::create_dir(&codex_config).unwrap();
        fs::create_dir(&override_destination).unwrap();
        let roots = resolve_configuration_roots(Some(root.path().to_path_buf()), None, None);
        let mut settings = Settings::default();
        settings
            .destinations
            .insert(Harness::Codex, codex_config.join("skills"));
        settings
            .destinations
            .insert(Harness::Claude, override_destination.clone());

        let descriptors = harness_descriptors(&settings, &roots);
        assert_eq!(
            descriptors.iter().map(|d| d.id).collect::<Vec<_>>(),
            Harness::ALL
        );
        assert!(descriptors[0].visible);
        assert!(!descriptors[0].destination_available);
        assert!(descriptors[1].visible);
        assert!(descriptors[1].destination_available);
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
        let roots = resolve_configuration_roots(None, None, None);
        let settings = Settings {
            destinations: [(Harness::Codex, override_destination)]
                .into_iter()
                .collect(),
            ..Settings::default()
        };
        let descriptors = harness_descriptors(&settings, &roots);
        assert!(descriptors[0].visible);
        assert!(descriptors[0].destination_available);
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
