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
    if settings::save(&app, settings)? {
        operations::invalidate_plans();
    }
    Ok(())
}

#[tauri::command]
fn open_harness_folder(app: tauri::AppHandle, harness: manager::Harness) -> Result<(), String> {
    let _guard = operations::acquire()?;
    let response = settings::load(&app);
    if let Some(error) = response.error {
        return Err(error);
    }

    let label = settings::harness_label(&response.settings, &harness)
        .ok_or_else(|| "Unknown harness identity".to_string())?;
    let path = settings::configured_destination(&response.settings, &harness)
        .ok_or_else(|| format!("{label} destination is not configured"))?;
    let target = folder_target(path)?;
    #[cfg(windows)]
    {
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
    #[cfg(target_os = "linux")]
    {
        open_linux_folder(&target, "xdg-open")
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Err(format!(
            "Opening {} is unsupported on this platform",
            target.display()
        ))
    }
}

#[cfg(target_os = "linux")]
fn open_linux_folder(target: &Path, program: impl AsRef<std::ffi::OsStr>) -> Result<(), String> {
    let status = std::process::Command::new(program)
        .arg(target)
        .status()
        .map_err(|error| format!("Could not open {} with xdg-open: {error}", target.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Could not open {}: xdg-open returned {status}",
            target.display()
        ))
    }
}

fn folder_target(path: &Path) -> Result<PathBuf, String> {
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
    on_progress: Option<tauri::ipc::JavaScriptChannelId>,
    webview: tauri::Webview,
    cache: tauri::State<'_, std::sync::Arc<std::sync::Mutex<manager::SourceCache>>>,
) -> Result<manager::ScanResponse, String> {
    let on_progress = on_progress.map(|id| id.channel_on::<_, manager::ScanProgress>(webview));
    let cache = cache.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operations::acquire()?;
        let response = settings::load(&app);
        if let Some(error) = response.error {
            return Err(error);
        }
        let generation = operations::begin_scan();
        let settings = response.settings;
        let response = manager::scan_cached_progress(
            &settings,
            &harness,
            &mut cache.lock().unwrap_or_else(|e| e.into_inner()),
            reuse_source,
            &|event| {
                if let Some(channel) = &on_progress {
                    let _ = channel.send(event);
                }
            },
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

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_folder_opener_passes_one_literal_argument_and_reports_failures() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("skills folder; $(literal)");
        fs::create_dir(&folder).unwrap();
        let target = folder_target(&folder).unwrap();
        let opener = root.path().join("opener");
        fs::write(
            &opener,
            b"#!/bin/sh\n[ \"$#\" -eq 1 ] || exit 2\nprintf '%s' \"$1\" > \"$0.argument\"\n",
        )
        .unwrap();
        fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
        open_linux_folder(&target, &opener).unwrap();
        assert_eq!(
            fs::read(root.path().join("opener.argument")).unwrap(),
            target.as_os_str().as_encoded_bytes()
        );
        fs::write(&opener, b"#!/bin/sh\nexit 3\n").unwrap();
        assert!(open_linux_folder(&target, &opener)
            .unwrap_err()
            .contains("returned"));
        assert!(open_linux_folder(&target, root.path().join("missing"))
            .unwrap_err()
            .contains("xdg-open"));
    }

    #[test]
    fn folder_target_requires_an_accessible_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("skills folder");
        fs::create_dir(&folder).unwrap();
        let target = folder_target(&folder).unwrap();
        assert!(target.is_dir());
        assert!(target.to_string_lossy().contains("skills folder"));

        let missing = root.path().join("missing");
        assert!(folder_target(&missing).unwrap_err().contains("unavailable"));
        let file = root.path().join("file");
        fs::write(&file, "content").unwrap();
        assert!(folder_target(&file)
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
