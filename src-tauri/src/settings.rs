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
}

pub fn defaults() -> Settings {
    let mut settings = Settings::default();
    for harness in Harness::ALL {
        if let Some(path) = default_destination(harness) {
            settings.destinations.insert(harness, path);
        }
    }
    settings
}

fn default_destination(harness: Harness) -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))?;
    let path = match harness {
        Harness::Codex => std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"))
            .join("skills"),
        Harness::Claude => home.join(".claude").join("skills"),
        Harness::Antigravity => home.join(".gemini").join("config").join("skills"),
        Harness::OpenCode => std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("opencode")
            .join("skills"),
    };
    Some(path)
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
            return SettingsResponse {
                settings: defaults(),
                settings_file: PathBuf::new(),
                error: Some(error),
            };
        }
    };
    let (settings, error) = load_file(&file);
    SettingsResponse {
        settings,
        settings_file: file,
        error,
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
        let settings = defaults();
        assert_eq!(settings.destinations.len(), 4);
        assert_eq!(settings.theme, Theme::Dark);
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
