use tauri::{AppHandle, Manager, PhysicalPosition};

const POPUP_WIDTH: u32 = 300;
const POPUP_HEIGHT: u32 = 300;
const POPUP_MARGIN_BOTTOM: u32 = 0;

/// Position and resize the popup window at bottom-center of the primary monitor.
/// The window is declared in tauri.conf.json (transparent, hidden).
pub fn setup_popup_position(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window("popup") {
        let (x, y) = get_bottom_center_position(app);
        let _ = popup.set_position(PhysicalPosition::new(x, y));
        log::info!(
            "Popup sized to {}x{} at ({}, {})",
            POPUP_WIDTH,
            POPUP_HEIGHT,
            x,
            y
        );
    }
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
    (
        (1920 - POPUP_WIDTH as i32) / 2,
        1080 - POPUP_HEIGHT as i32 - POPUP_MARGIN_BOTTOM as i32,
    )
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
