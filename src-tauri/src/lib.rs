mod manager;
mod operations;
mod settings;

use std::{
    fs,
    path::{Path, PathBuf},
};

#[tauri::command]
fn load_settings(app: tauri::AppHandle) -> settings::SettingsResponse {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: settings::Settings) -> Result<(), String> {
    let _guard = operations::acquire()?;
    settings::save(&app, settings)?;
    operations::invalidate_plans();
    Ok(())
}

#[tauri::command]
fn open_harness_folder(app: tauri::AppHandle, harness: manager::Harness) -> Result<(), String> {
    let _guard = operations::acquire()?;
    let response = settings::load(&app);
    if let Some(error) = response.error {
        return Err(error);
    }

    #[cfg(windows)]
    {
        let path = settings::configured_destination(&response.settings, harness)
            .ok_or_else(|| format!("{} destination is not configured", harness.label()))?;
        let target = explorer_target(path)?;
        std::process::Command::new("explorer.exe")
            .arg(&target)
            .spawn()
            .map(|_| ())
            .map_err(|error| {
                format!(
                    "Could not open {} in File Explorer: {error}",
                    target.display()
                )
            })
    }
    #[cfg(not(windows))]
    {
        Err("Opening harness folders in File Explorer is only supported on Windows".to_string())
    }
}

fn explorer_target(path: &Path) -> Result<PathBuf, String> {
    let target = path.canonicalize().map_err(|error| {
        format!(
            "Harness destination is unavailable: {}: {error}",
            path.display()
        )
    })?;
    if !target.is_dir() {
        return Err(format!(
            "Harness destination is not a directory: {}",
            path.display()
        ));
    }
    fs::read_dir(&target).map_err(|error| {
        format!(
            "Harness destination is not accessible: {}: {error}",
            path.display()
        )
    })?;
    Ok(normalize_explorer_target(target))
}

#[cfg(windows)]
fn normalize_explorer_target(path: PathBuf) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    const UNC_PREFIX: &[u16] = &[92, 92, 63, 92, 85, 78, 67, 92];
    const VERBATIM_PREFIX: &[u16] = &[92, 92, 63, 92];
    if wide.starts_with(UNC_PREFIX) {
        let mut normalized = vec![92, 92];
        normalized.extend_from_slice(&wide[UNC_PREFIX.len()..]);
        PathBuf::from(std::ffi::OsString::from_wide(&normalized))
    } else if wide.starts_with(VERBATIM_PREFIX)
        && wide.get(5) == Some(&(b':' as u16))
        && matches!(wide[4], 65..=90 | 97..=122)
    {
        PathBuf::from(std::ffi::OsString::from_wide(
            &wide[VERBATIM_PREFIX.len()..],
        ))
    } else {
        path
    }
}

#[cfg(not(windows))]
fn normalize_explorer_target(path: PathBuf) -> PathBuf {
    path
}

#[tauri::command]
async fn scan_skills(
    app: tauri::AppHandle,
    harness: manager::Harness,
    reuse_source: bool,
    cache: tauri::State<'_, std::sync::Arc<std::sync::Mutex<manager::SourceCache>>>,
) -> Result<manager::ScanResponse, String> {
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operations::acquire()?;
        let response = settings::load(&app);
        if let Some(error) = response.error {
            return Err(error);
        }
        let generation = operations::begin_scan();
        let settings = response.settings;
        let response = manager::scan_cached(
            &settings,
            harness,
            &mut cache.lock().unwrap_or_else(|e| e.into_inner()),
            reuse_source,
        )?;
        operations::record_scan(&response, generation);
        Ok(response)
    })
    .await
    .map_err(|error| format!("Skill scan could not finish: {error}"))?
}

#[tauri::command]
async fn prepare_operation(
    app: tauri::AppHandle,
    harness: manager::Harness,
    action: operations::OperationAction,
    revision: String,
    selected: Vec<String>,
) -> Result<operations::PrepareResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operations::acquire()?;
        let response = settings::load(&app);
        if let Some(error) = response.error {
            return Err(error);
        }
        operations::prepare_operation(&response.settings, harness, &revision, action, &selected)
    })
    .await
    .map_err(|error| format!("Operation preparation could not finish: {error}"))?
}

#[tauri::command]
async fn execute_operation(
    app: tauri::AppHandle,
    token: String,
    acknowledge_links: bool,
    on_event: tauri::ipc::Channel<operations::OperationEvent>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operations::acquire()?;
        let response = settings::load(&app);
        if let Some(error) = response.error {
            return Err(error);
        }
        operations::execute_locked(&token, &response.settings, acknowledge_links, on_event)
    })
    .await
    .map_err(|error| format!("Operation could not finish: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(std::sync::Arc::new(std::sync::Mutex::new(
            manager::SourceCache::default(),
        )))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            open_harness_folder,
            scan_skills,
            prepare_operation,
            execute_operation
        ])
        .on_window_event(|_, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if operations::is_running() {
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running AI Skill Manager");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_target_requires_an_accessible_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("skills folder");
        fs::create_dir(&folder).unwrap();
        let target = explorer_target(&folder).unwrap();
        assert!(target.is_dir());
        assert!(target.to_string_lossy().contains("skills folder"));

        let missing = root.path().join("missing");
        assert!(explorer_target(&missing)
            .unwrap_err()
            .contains("unavailable"));
        let file = root.path().join("file");
        fs::write(&file, "content").unwrap();
        assert!(explorer_target(&file)
            .unwrap_err()
            .contains("not a directory"));
    }

    #[cfg(windows)]
    #[test]
    fn explorer_target_normalizes_verbatim_drive_and_unc_paths() {
        use std::os::windows::ffi::OsStringExt;
        let drive = PathBuf::from(std::ffi::OsString::from_wide(
            &"\\\\?\\C:\\Skills Folder"
                .encode_utf16()
                .collect::<Vec<_>>(),
        ));
        let unc = PathBuf::from(std::ffi::OsString::from_wide(
            &"\\\\?\\UNC\\server\\share\\Skills Folder"
                .encode_utf16()
                .collect::<Vec<_>>(),
        ));
        assert_eq!(
            normalize_explorer_target(drive),
            PathBuf::from("C:\\Skills Folder")
        );
        assert_eq!(
            normalize_explorer_target(unc),
            PathBuf::from("\\\\server\\share\\Skills Folder")
        );
    }
}
