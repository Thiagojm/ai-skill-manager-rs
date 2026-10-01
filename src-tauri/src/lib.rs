mod manager;
mod operations;
mod settings;

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
async fn scan_skills(
    app: tauri::AppHandle,
    harness: manager::Harness,
) -> Result<manager::ScanResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = operations::acquire()?;
        let response = settings::load(&app);
        if let Some(error) = response.error {
            return Err(error);
        }
        let generation = operations::begin_scan();
        let settings = response.settings;
        let response = manager::scan(&settings, harness)?;
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
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
