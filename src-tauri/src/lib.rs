mod manager;
mod settings;

#[tauri::command]
fn load_settings(app: tauri::AppHandle) -> settings::SettingsResponse {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(app: tauri::AppHandle, settings: settings::Settings) -> Result<(), String> {
    settings::save(&app, settings)
}

#[tauri::command]
async fn scan_skills(
    app: tauri::AppHandle,
    harness: manager::Harness,
) -> Result<manager::ScanResponse, String> {
    let response = settings::load(&app);
    if let Some(error) = response.error {
        return Err(error);
    }
    let settings = response.settings;
    tauri::async_runtime::spawn_blocking(move || manager::scan(&settings, harness))
        .await
        .map_err(|error| format!("Skill scan could not finish: {error}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            scan_skills
        ])
        .run(tauri::generate_context!())
        .expect("error while running AI Skill Manager");
}
