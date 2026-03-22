use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager};
use tauri_utils::config::WebviewUrl;

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_internal(&app);
    Ok(())
}

pub fn open_settings_internal(app: &AppHandle) {
    // If settings window already exists, just focus it
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_focus();
        return;
    }

    let _ = WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("settings.html".into()),
    )
    .title("Murmur Settings")
    .inner_size(480.0, 560.0)
    .resizable(false)
    .center()
    .build();
}
