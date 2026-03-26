use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager, WebviewUrl};
use tauri_utils::config::Color;

const POPUP_WIDTH: u32 = 280;
const POPUP_HEIGHT: u32 = 48;
const POPUP_MARGIN_BOTTOM: u32 = 40;

/// Create the popup window (hidden initially). Call once during setup.
pub fn create_popup_window(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("popup").is_some() {
        return Ok(());
    }

    let (x, y) = get_bottom_center_position(app);

    let _window = WebviewWindowBuilder::new(
        app,
        "popup",
        WebviewUrl::App("popup.html".into()),
    )
    .title("Murmur Recording")
    .inner_size(POPUP_WIDTH as f64, POPUP_HEIGHT as f64)
    .position(x as f64, y as f64)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .background_color(Color(0, 0, 0, 0)) // Fully transparent WebView background
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;

    log::info!("Popup window created at ({}, {})", x, y);
    Ok(())
}

fn get_bottom_center_position(app: &AppHandle) -> (i32, i32) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = window.primary_monitor() {
            let size = monitor.size();
            let pos = monitor.position();
            let x = pos.x + (size.width as i32 - POPUP_WIDTH as i32) / 2;
            let y = pos.y + size.height as i32 - POPUP_HEIGHT as i32 - POPUP_MARGIN_BOTTOM as i32;
            return (x, y);
        }
    }
    // Fallback: assume 1920x1080
    ((1920 - POPUP_WIDTH as i32) / 2, 1080 - POPUP_HEIGHT as i32 - POPUP_MARGIN_BOTTOM as i32)
}

/// Show the popup if the main skin is hidden.
pub fn show_popup(app: &AppHandle) {
    if !should_show_popup(app) {
        return;
    }
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.show();
        let _ = popup.set_focus();
    }
}

/// Hide the popup.
pub fn hide_popup(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
}

/// Popup should show only when the main skin window is not visible.
fn should_show_popup(app: &AppHandle) -> bool {
    if let Some(main_win) = app.get_webview_window("main") {
        !main_win.is_visible().unwrap_or(true)
    } else {
        true
    }
}
