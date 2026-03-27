mod audio;
mod commands;
mod inject;
mod state;
mod stt;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
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
            // --- Detect display server and start injection subsystem ---
            let display_server = inject::display_server::detect();
            log::info!("Display server: {}", display_server);
            let _ = app.handle().emit("display-server", serde_json::json!({
                "type": format!("{}", display_server)
            }));

            // Warn about missing injection tools
            match display_server {
                inject::display_server::DisplayServer::X11 => {
                    if !inject::paste::is_xdotool_available() {
                        let _ = app.handle().emit("system-warning", serde_json::json!({
                            "message": "xdotool not found. Text will be copied to clipboard only — install xdotool for direct typing."
                        }));
                    }
                }
                inject::display_server::DisplayServer::Wayland => {
                    // wtype is checked at injection time, not startup
                    log::info!("Wayland session — will use wtype for text injection.");
                }
                inject::display_server::DisplayServer::Unknown => {
                    let _ = app.handle().emit("system-warning", serde_json::json!({
                        "message": "Could not detect display server. Text will be copied to clipboard only."
                    }));
                }
            }

            inject::paste::start_window_tracker();

            // --- Load settings from store into AppState ---
            let (hotkey, active_model) = {
                let handle = app.handle().clone();
                match handle.store("settings.json") {
                    Ok(store) => {
                        let hk: Option<serde_json::Value> = store.get("hotkey");
                        let hotkey = hk.and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_else(|| DEFAULT_HOTKEY.to_string());
                        let model: Option<serde_json::Value> = store.get("model");
                        let active_model = model.and_then(|v| v.as_str().map(String::from))
                            .unwrap_or_else(|| "ggml-tiny.en.bin".to_string());
                        (hotkey, active_model)
                    }
                    Err(_) => (DEFAULT_HOTKEY.to_string(), "ggml-tiny.en.bin".to_string()),
                }
            };

            // Cache active model in AppState
            if let Ok(mut inner) = app.state::<state::AppState>().lock() {
                inner.active_model = active_model;
            }

            if let Err(e) = commands::hotkey::register_hotkey(app.handle(), &hotkey) {
                log::error!("Failed to register hotkey '{}': {}", hotkey, e);
                // Fallback to default if custom hotkey fails
                if hotkey != DEFAULT_HOTKEY {
                    let _ = commands::hotkey::register_hotkey(app.handle(), DEFAULT_HOTKEY);
                }
            }

            // --- Hide skin on startup if showSkin is false ---
            let show_skin = {
                let handle = app.handle().clone();
                match handle.store("settings.json") {
                    Ok(store) => store.get("showSkin")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    Err(_) => true,
                }
            };
            if !show_skin {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // --- Position popup window (declared in tauri.conf.json, hidden) ---
            commands::popup::setup_popup_position(app.handle());

            // --- Listen for auto-stop from capture thread (VAD / max duration) ---
            let handle_for_autostop = app.handle().clone();
            app.listen("capture-auto-stopped", move |_| {
                log::debug!("Capture auto-stopped — triggering full stop flow");
                let _ = commands::audio::stop_recording_internal(handle_for_autostop.clone());
            });

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
                            let was_visible = window.is_visible().unwrap_or(false);
                            if was_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            // Persist to store and emit event
                            if let Ok(store) = app.store("settings.json") {
                                let _ = store.set("showSkin", serde_json::json!(!was_visible));
                                let _ = store.save();
                            }
                            let _ = app.emit("skin-visibility-changed", serde_json::json!({ "visible": !was_visible }));
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
                            // Persist visibility
                            if let Ok(store) = app.store("settings.json") {
                                let _ = store.set("showSkin", serde_json::json!(true));
                                let _ = store.save();
                            }
                            let _ = app.emit("skin-visibility-changed", serde_json::json!({ "visible": true }));
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
