mod audio;
pub mod commands;
mod inject;
pub mod state;
mod stt;

// Public library surface. Consumed by this crate's binary and by external
// consumers (integration tests, downstream CLI tools).
pub use audio::capture::{start_capture, AudioLevel, AudioLevelCallback, AutoStopCallback};
pub use commands::audio::trim_trailing_silence;
pub use inject::display_server::{self as display_server, DisplayServer};
pub use inject::paste::{get_last_external_window, paste_text, sanitise_for_injection};
pub use stt::engine::{SttConfig, SttEngine};
pub use stt::local_whisper::LocalWhisperEngine;
pub use stt::model_manager::{
    download_model_by_name, get_model_path, list_available_models, models_dir,
    ModelDownloadProgress, ModelEntry, ModelInfo, ProgressCallback, MODELS,
};
pub use stt::whisper::{clear_cache, resample, transcribe};

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
};
use tauri_plugin_store::StoreExt;

const DEFAULT_HOTKEY: &str = "Ctrl+Shift+Space";
const TRAY_ID: &str = "murmur-tray";

/// Shared setup logic invoked by both the free desktop binary (`run_free`)
/// and the Pro desktop binary (in `murmur-pro`). Registers updater plugin,
/// detects display server, wires hotkeys, loads settings, launches
/// onboarding when needed, positions popup, sets up tray icon + menu,
/// and registers auto-stop and recording-state listeners.
pub fn shared_setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // --- Updater plugin (registers inside setup, not on builder) ---
    app.handle()
        .plugin(tauri_plugin_updater::Builder::new().build())?;

    // --- Check for updates in background ---
    let update_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        match tauri_plugin_updater::UpdaterExt::updater(&update_handle) {
            Ok(updater) => match updater.check().await {
                Ok(Some(update)) => {
                    log::info!(
                        "Update available: v{} (current: {})",
                        update.version,
                        update.current_version
                    );
                    let _ = update_handle.emit(
                        "update-available",
                        serde_json::json!({
                            "version": update.version,
                            "current": update.current_version,
                        }),
                    );
                }
                Ok(None) => log::debug!("No updates available"),
                Err(e) => log::debug!("Update check failed (non-fatal): {}", e),
            },
            Err(e) => log::debug!("Updater init failed (non-fatal): {}", e),
        }
    });

    // --- Detect display server and start injection subsystem ---
    let display_server = inject::display_server::detect();
    log::info!("Display server: {}", display_server);
    let _ = app.handle().emit(
        "display-server",
        serde_json::json!({
            "type": format!("{}", display_server)
        }),
    );

    // Warn about missing injection tools
    match display_server {
        inject::display_server::DisplayServer::X11 => {
            if !inject::paste::is_xdotool_available() {
                let _ = app.handle().emit("system-warning", serde_json::json!({
                            "message": "xdotool not found. Text will be copied to clipboard only -- install xdotool for direct typing."
                        }));
            }
        }
        inject::display_server::DisplayServer::Wayland => {
            // wtype is checked at injection time, not startup
            log::info!("Wayland session -- will use wtype for text injection.");
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
                let hotkey = hk
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| DEFAULT_HOTKEY.to_string());
                let model: Option<serde_json::Value> = store.get("model");
                let active_model = model
                    .and_then(|v| v.as_str().map(String::from))
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

    // --- Tray-only: always hide main window on startup ---
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    // --- First-run onboarding wizard ---
    let onboarding_complete = {
        let handle = app.handle().clone();
        match handle.store("settings.json") {
            Ok(store) => store
                .get("onboardingComplete")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            Err(_) => false,
        }
    };
    if !onboarding_complete {
        // Hide main window, show onboarding wizard
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        commands::settings::open_onboarding_internal(app.handle());
    }

    // --- Position popup window (declared in tauri.conf.json, hidden) ---
    commands::popup::setup_popup_position(app.handle());

    // --- Listen for onboarding completion ---
    app.listen("onboarding-complete", move |_| {
        log::info!("Onboarding complete -- tray-only, main window stays hidden");
    });

    // --- Listen for auto-stop from capture thread (VAD / max duration) ---
    let handle_for_autostop = app.handle().clone();
    app.listen("capture-auto-stopped", move |_| {
        log::debug!("Capture auto-stopped -- triggering full stop flow");
        let _ = commands::audio::stop_recording_internal(handle_for_autostop.clone());
    });

    // --- System Tray ---
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Murmur", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&settings_item, &sep, &quit_item])?;

    let Some(icon) = app.default_window_icon().cloned() else {
        return Err("No default window icon configured in tauri.conf.json".into());
    };

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Murmur -- Voice to Text")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
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
                commands::settings::open_settings_internal(app);
            }
        })
        .build(app)?;

    // --- Tray icon + tooltip driven by recording state and live mic level ---
    //
    // Three visible states with five amplitude variants for the recording
    // glow that responds to the user's voice in real time:
    //
    //   idle       -> brand icon                            + "Murmur -- Voice to Text"
    //   recording  -> red record dot with pulsing glow      + "Murmur -- Recording..."
    //                 (5 variants 0..4 driven by mic RMS)
    //   processing -> blue ring                             + "Murmur -- Processing..."
    //
    // On Wayland this is the only recording indicator (the popup pill is
    // suppressed there because Wayland forbids absolute window positioning,
    // see murmurlinux/internal#136). On X11 the pill still shows; the
    // tray animation is additive.
    let idle_icon = std::sync::Arc::new(
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
            .expect("idle tray icon failed to load"),
    );
    let recording_icons: std::sync::Arc<[tauri::image::Image]> = std::sync::Arc::from([
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon-recording-0.png"))
            .expect("recording-0 icon failed to load"),
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon-recording-1.png"))
            .expect("recording-1 icon failed to load"),
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon-recording-2.png"))
            .expect("recording-2 icon failed to load"),
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon-recording-3.png"))
            .expect("recording-3 icon failed to load"),
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon-recording-4.png"))
            .expect("recording-4 icon failed to load"),
    ]);
    let processing_icon = std::sync::Arc::new(
        tauri::image::Image::from_bytes(include_bytes!("../icons/icon-processing.png"))
            .expect("processing tray icon failed to load"),
    );

    // Shared across both listeners. STATE_RECORDING (1) is the only state
    // that lets the audio-level handler touch the icon.
    const STATE_IDLE: u8 = 0;
    const STATE_RECORDING: u8 = 1;
    const STATE_PROCESSING: u8 = 2;

    let current_state = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(STATE_IDLE));
    let last_level_update_ms = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    {
        let handle = app.handle().clone();
        let state = current_state.clone();
        let idle = idle_icon.clone();
        let recording = recording_icons.clone();
        let processing = processing_icon.clone();
        app.listen("recording-state", move |event| {
            let payload_state = serde_json::from_str::<serde_json::Value>(event.payload())
                .ok()
                .and_then(|v| v.get("state").and_then(|s| s.as_str().map(String::from)))
                .unwrap_or_default();

            let (code, icon_clone, tooltip): (u8, tauri::image::Image, &str) =
                match payload_state.as_str() {
                    "recording" => (
                        STATE_RECORDING,
                        recording[0].clone(),
                        "Murmur -- Recording...",
                    ),
                    "processing" => (
                        STATE_PROCESSING,
                        (*processing).clone(),
                        "Murmur -- Processing...",
                    ),
                    _ => (STATE_IDLE, (*idle).clone(), "Murmur -- Voice to Text"),
                };

            state.store(code, std::sync::atomic::Ordering::Relaxed);
            if let Some(tray) = handle.tray_by_id(TRAY_ID) {
                let _ = tray.set_icon(Some(icon_clone));
                let _ = tray.set_tooltip(Some(tooltip));
            }
        });
    }

    // audio-level fires at ~60Hz from the capture thread. Throttle tray
    // updates to ~10Hz so the kernel and tray bus are not overloaded.
    {
        let handle = app.handle().clone();
        let state = current_state.clone();
        let last_ms = last_level_update_ms.clone();
        let recording = recording_icons.clone();
        app.listen("audio-level", move |event| {
            if state.load(std::sync::atomic::Ordering::Relaxed) != STATE_RECORDING {
                return;
            }
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let prev = last_ms.load(std::sync::atomic::Ordering::Relaxed);
            if now_ms.saturating_sub(prev) < 100 {
                return;
            }
            last_ms.store(now_ms, std::sync::atomic::Ordering::Relaxed);

            let rms = serde_json::from_str::<serde_json::Value>(event.payload())
                .ok()
                .and_then(|v| v.get("rms").and_then(|r| r.as_f64()))
                .unwrap_or(0.0);

            // Empirical thresholds tuned for normal speech-volume RMS in
            // 0.0-1.0. Quiet rooms sit around 0.02; conversational
            // dictation is 0.08-0.20; loud emphasis 0.30+.
            let variant = if rms < 0.03 {
                0
            } else if rms < 0.08 {
                1
            } else if rms < 0.18 {
                2
            } else if rms < 0.32 {
                3
            } else {
                4
            };

            if let Some(tray) = handle.tray_by_id(TRAY_ID) {
                let _ = tray.set_icon(Some(recording[variant].clone()));
            }
        });
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_free() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_secs()
        .init();

    let mut builder = tauri::Builder::default()
        .manage(state::AppState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    // On Wayland the X11 hotkey plugin is bypassed: its set_event_handler call
    // would swallow events from the portal-backed path used by hotkey_wayland.
    if !is_wayland_session() {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .invoke_handler(tauri::generate_handler![
            commands::audio::start_recording,
            commands::audio::stop_recording,
            commands::settings::open_settings,
            commands::hotkey::change_hotkey,
            commands::models::list_models,
            commands::models::download_model,
            commands::models::set_active_model,
            commands::settings::set_start_on_login,
            commands::settings::check_microphone,
            commands::settings::list_microphones,
            commands::settings::start_mic_test,
        ])
        .setup(shared_setup)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    inject::display_server::detect() == inject::display_server::DisplayServer::Wayland
}

#[cfg(not(target_os = "linux"))]
fn is_wayland_session() -> bool {
    false
}
