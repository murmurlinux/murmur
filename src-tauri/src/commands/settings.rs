use std::fs;
use std::path::PathBuf;
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager};
use tauri_utils::config::{Color, WebviewUrl};

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_internal(&app);
    Ok(())
}

/// Set or remove the XDG autostart .desktop entry for Murmur.
#[tauri::command]
pub fn set_start_on_login(enabled: bool) -> Result<(), String> {
    let autostart_dir = dirs_autostart().ok_or("Could not determine autostart directory")?;
    let desktop_file = autostart_dir.join("murmur.desktop");

    if enabled {
        fs::create_dir_all(&autostart_dir).map_err(|e| e.to_string())?;
        let exec_path = std::env::current_exe().map_err(|e| e.to_string())?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Murmur\nComment=AI voice-to-text\nExec={}\nIcon=murmur\nTerminal=false\nStartupWMClass=murmur\nX-GNOME-Autostart-enabled=true\n",
            exec_path.display()
        );
        fs::write(&desktop_file, content).map_err(|e| e.to_string())?;
        log::info!("Autostart enabled: {}", desktop_file.display());
    } else if desktop_file.exists() {
        fs::remove_file(&desktop_file).map_err(|e| e.to_string())?;
        log::info!("Autostart disabled: removed {}", desktop_file.display());
    }

    Ok(())
}

fn dirs_autostart() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.config_dir().join("autostart"))
}

#[derive(serde::Serialize)]
pub struct MicrophoneInfo {
    pub name: String,
    pub available: bool,
}

/// Check if a microphone is available and return its name.
#[tauri::command]
pub fn check_microphone() -> MicrophoneInfo {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    match host.default_input_device() {
        Some(device) => MicrophoneInfo {
            name: device.name().unwrap_or_else(|_| "Unknown".to_string()),
            available: true,
        },
        None => MicrophoneInfo {
            name: "No microphone detected".to_string(),
            available: false,
        },
    }
}

pub fn open_onboarding_internal(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("onboarding") {
        let _ = window.set_focus();
        return;
    }

    let _ = WebviewWindowBuilder::new(app, "onboarding", WebviewUrl::App("onboarding.html".into()))
        .title("Welcome to Murmur")
        .inner_size(520.0, 620.0)
        .resizable(false)
        .background_color(Color(6, 13, 24, 255))
        .center()
        .build();
}

pub fn open_settings_internal(app: &AppHandle) {
    // If settings window already exists, just focus it
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_focus();
        return;
    }

    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings.html".into()))
        .title("Murmur Settings")
        .inner_size(480.0, 680.0)
        .min_inner_size(480.0, 400.0)
        .background_color(Color(6, 13, 24, 255)) // #060d18 -- ocean-deep, eliminates white flash
        .center()
        .build();
}
