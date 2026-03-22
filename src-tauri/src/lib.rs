mod audio;
mod commands;
mod inject;
mod state;
mod stt;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_store::StoreExt;

const DEFAULT_HOTKEY: &str = "Ctrl+Shift+Space";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::audio::start_recording,
            commands::audio::stop_recording,
            commands::settings::open_settings,
            commands::hotkey::change_hotkey,
            commands::models::list_models,
            commands::models::download_model,
            commands::models::set_active_model,
        ])
        .setup(|app| {
            // --- Window Tracker (tracks last non-Murmur focused window for text injection) ---
            inject::paste::start_window_tracker();

            // --- Register global hotkey (read from store or use default) ---
            let hotkey = {
                let handle = app.handle().clone();
                match handle.store("settings.json") {
                    Ok(store) => {
                        let val: Option<serde_json::Value> = store.get("hotkey");
                        val.and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_else(|| DEFAULT_HOTKEY.to_string())
                    }
                    Err(_) => DEFAULT_HOTKEY.to_string(),
                }
            };

            if let Err(e) = commands::hotkey::register_hotkey(app.handle(), &hotkey) {
                eprintln!("Failed to register hotkey '{}': {}", hotkey, e);
                // Fallback to default if custom hotkey fails
                if hotkey != DEFAULT_HOTKEY {
                    let _ = commands::hotkey::register_hotkey(app.handle(), DEFAULT_HOTKEY);
                }
            }

            // --- System Tray ---
            let show_item =
                MenuItem::with_id(app, "show_hide", "Show/Hide", true, None::<&str>)?;
            let aot_item = MenuItem::with_id(
                app,
                "always_on_top",
                "Always on Top",
                true,
                None::<&str>,
            )?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_item =
                MenuItem::with_id(app, "quit", "Quit Murmur", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&show_item, &aot_item, &sep, &settings_item, &sep2, &quit_item],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("No default window icon configured in tauri.conf.json").clone())
                .tooltip("Murmur — Voice to Text")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show_hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                    "always_on_top" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_on_top = window.is_always_on_top().unwrap_or(false);
                            let _ = window.set_always_on_top(!is_on_top);
                        }
                    }
                    "settings" => {
                        commands::settings::open_settings_internal(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
