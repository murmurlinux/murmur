use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_store::StoreExt;

use crate::commands::audio;
use crate::commands::settings;
use crate::state::{AppState, RecordingState};

/// The global shortcut that opens the settings window.
pub const SETTINGS_SHORTCUT: &str = "Ctrl+Shift+,";

/// Read the recordMode setting from the store ("hold" or "tap").
fn get_record_mode(app: &AppHandle) -> String {
    app.store("settings.json")
        .ok()
        .and_then(|store| store.get("recordMode"))
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "hold".to_string())
}

/// Check if currently recording (for tap-to-toggle).
fn is_recording(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .lock()
        .map(|inner| inner.recording_state == RecordingState::Recording)
        .unwrap_or(false)
}

/// Register the recording hotkey with the given shortcut string.
/// Called on startup and when the user changes the hotkey.
///
/// This unregisters all shortcuts and re-registers both the recording
/// hotkey and the settings shortcut, so that both coexist.
pub fn register_hotkey(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    let gs = app.global_shortcut();

    // Unregister any existing shortcuts first
    gs.unregister_all().map_err(|e| e.to_string())?;

    // Register the recording hotkey
    gs.on_shortcut(shortcut, move |app, _shortcut, event| {
        let mode = get_record_mode(app);

        match event.state {
            ShortcutState::Pressed => {
                if mode == "tap" {
                    if is_recording(app) {
                        log::debug!("Hotkey tap -- stopping recording");
                        let _ = audio::stop_recording_internal(app.clone());
                    } else {
                        log::debug!("Hotkey tap -- starting recording");
                        let _ = audio::start_recording_internal(app.clone());
                    }
                } else {
                    log::debug!("Hotkey pressed -- starting recording");
                    let _ = audio::start_recording_internal(app.clone());
                }
            }
            ShortcutState::Released => {
                if mode != "tap" {
                    log::debug!("Hotkey released -- stopping recording");
                    let _ = audio::stop_recording_internal(app.clone());
                }
            }
        }
    })
    .map_err(|e| format!("Failed to register hotkey '{}': {}", shortcut, e))?;

    log::info!("Hotkey registered: {}", shortcut);

    // Re-register the settings shortcut alongside the recording hotkey
    register_settings_shortcut(app)?;

    Ok(())
}

/// Register the global shortcut that opens the settings window.
pub fn register_settings_shortcut(app: &AppHandle) -> Result<(), String> {
    let gs = app.global_shortcut();

    gs.on_shortcut(SETTINGS_SHORTCUT, move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            log::debug!("Settings shortcut pressed -- opening settings");
            settings::open_settings_internal(app);
        }
    })
    .map_err(|e| {
        format!(
            "Failed to register settings shortcut '{}': {}",
            SETTINGS_SHORTCUT, e
        )
    })?;

    log::info!("Settings shortcut registered: {}", SETTINGS_SHORTCUT);
    Ok(())
}

#[tauri::command]
pub fn change_hotkey(app: AppHandle, new_hotkey: String) -> Result<(), String> {
    register_hotkey(&app, &new_hotkey)
}
