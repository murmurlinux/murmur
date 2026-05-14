// Tauri commands for BYOK key management.
//
// The raw key NEVER crosses IPC after the user pastes it. `byok_set_key`
// takes the key one-way (frontend -> Rust); the inverse is deliberately
// absent. The UI uses `byok_has_key` / `byok_key_hint` to render state
// without ever holding the cleartext key in JS again.

use crate::byok_storage::ByokStorage;
use tauri::State;

#[tauri::command]
pub fn byok_storage_mode(storage: State<'_, ByokStorage>) -> &'static str {
    storage.mode().as_str()
}

#[tauri::command]
pub fn byok_set_key(
    provider: String,
    key: String,
    storage: State<'_, ByokStorage>,
) -> Result<(), String> {
    storage.set_key(&provider, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn byok_clear_key(provider: String, storage: State<'_, ByokStorage>) -> Result<(), String> {
    storage.clear_key(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn byok_has_key(provider: String, storage: State<'_, ByokStorage>) -> Result<bool, String> {
    storage.has_key(&provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn byok_key_hint(
    provider: String,
    storage: State<'_, ByokStorage>,
) -> Result<Option<String>, String> {
    storage.key_hint(&provider).map_err(|e| e.to_string())
}
