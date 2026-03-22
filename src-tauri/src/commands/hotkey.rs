use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands::audio;

/// Register the recording hotkey with the given shortcut string.
/// Called on startup and when the user changes the hotkey.
pub fn register_hotkey(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    let gs = app.global_shortcut();

    // Unregister any existing shortcuts first
    gs.unregister_all().map_err(|e| e.to_string())?;

    gs.on_shortcut(shortcut, move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            log::debug!("Hotkey pressed — starting recording");
            let _ = audio::start_recording_internal(app.clone());
        } else if event.state == ShortcutState::Released {
            log::debug!("Hotkey released — stopping recording");
            let _ = audio::stop_recording_internal(app.clone());
        }
    })
    .map_err(|e| format!("Failed to register hotkey '{}': {}", shortcut, e))?;

    log::info!("Hotkey registered: {}", shortcut);
    Ok(())
}

#[tauri::command]
pub fn change_hotkey(app: AppHandle, new_hotkey: String) -> Result<(), String> {
    register_hotkey(&app, &new_hotkey)
}
