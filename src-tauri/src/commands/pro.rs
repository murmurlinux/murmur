// Tauri commands exposing Pro entitlement state to the frontend. The
// JWT itself never crosses IPC — only the derived booleans / display
// strings. See pro_state.rs for the verification + persistence logic.

use crate::pro_state::{ProState, SIGN_IN_URL};
use tauri::{AppHandle, State, Wry};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn pro_is_active(state: State<'_, ProState>) -> bool {
    state.is_active()
}

#[tauri::command]
pub fn pro_email(state: State<'_, ProState>) -> Option<String> {
    state.email()
}

#[tauri::command]
pub fn pro_expires_at(state: State<'_, ProState>) -> Option<String> {
    state.pro_expires_at()
}

#[tauri::command]
pub fn pro_sign_out(app: AppHandle<Wry>, state: State<'_, ProState>) -> Result<(), String> {
    state.sign_out(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pro_open_sign_in(app: AppHandle<Wry>) -> Result<(), String> {
    app.opener()
        .open_url(SIGN_IN_URL, None::<&str>)
        .map_err(|e| e.to_string())
}
