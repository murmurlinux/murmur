//! Wayland push-to-talk hotkey via `org.freedesktop.portal.GlobalShortcuts`.
//!
//! On native Wayland sessions the X11 `XGrabKey` path used by
//! `tauri-plugin-global-shortcut` cannot see hardware key events for
//! non-focused windows. This module registers Murmur's actions through the
//! GlobalShortcuts portal instead, which surfaces a one-time system consent
//! dialog and then delivers press/release events for the user-bound key.
//!
//! Implementation tracks `tauri-apps/global-hotkey#162`. When that PR merges
//! upstream, replace this module with the standard plugin path and drop the
//! `[patch.crates-io]` entry in `Cargo.toml`.

use std::str::FromStr;
use std::sync::OnceLock;

use global_hotkey::{
    hotkey::HotKey,
    wayland::{WlHotKeysChangedEvent, WlNewHotKeyAction},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::commands::{audio, settings};
use crate::state::{AppState, RecordingState};

/// Reverse-DNS app id. Must match `tauri.conf.json` `identifier`, the
/// installed `.desktop` filename, and the D-Bus bus name. GNOME 48 has a
/// known consent re-prompt bug when these disagree.
const APP_ID: &str = "com.murmurlinux.murmur";

const PTT_ACTION_ID: u32 = 0;
const SETTINGS_ACTION_ID: u32 = 1;

const SETTINGS_HOTKEY: &str = "Ctrl+Shift+Comma";

/// Held for the lifetime of the process. Dropping it tears down the portal
/// session and unbinds every shortcut.
static MANAGER: OnceLock<GlobalHotKeyManager> = OnceLock::new();

/// Register PTT + Settings actions with the portal once and start the event
/// pump. Subsequent calls (e.g. when the user edits the hotkey in the in-app
/// settings panel) are no-ops: on Wayland the binding is owned by the system
/// settings UI, not by Murmur. The user changes their key from
/// `Settings → Keyboard → Custom Shortcuts`.
pub fn register(app: &AppHandle, ptt_hotkey: &str) -> Result<(), String> {
    if MANAGER.get().is_some() {
        eprintln!(
            "[murmur:wayland] hotkeys already registered; ignoring repeat call (preference '{}')",
            ptt_hotkey
        );
        return Ok(());
    }

    eprintln!("[murmur:wayland] init: creating GlobalHotKeyManager");
    let manager = GlobalHotKeyManager::new().map_err(|e| {
        eprintln!("[murmur:wayland] GlobalHotKeyManager::new failed: {}", e);
        format!("Failed to create GlobalHotKeyManager: {}", e)
    })?;

    let preferred_ptt = HotKey::from_str(ptt_hotkey).ok();
    if preferred_ptt.is_none() {
        eprintln!(
            "[murmur:wayland] WARN: could not parse PTT hotkey '{}' (portal will prompt with no default)",
            ptt_hotkey
        );
    }
    let preferred_settings = HotKey::from_str(SETTINGS_HOTKEY).ok();

    let actions = [
        WlNewHotKeyAction::new(PTT_ACTION_ID, "Push to talk", preferred_ptt),
        WlNewHotKeyAction::new(
            SETTINGS_ACTION_ID,
            "Open Murmur settings",
            preferred_settings,
        ),
    ];

    eprintln!(
        "[murmur:wayland] calling wl_register_all(app_id={}) with {} actions",
        APP_ID,
        actions.len()
    );
    manager.wl_register_all(APP_ID, &actions).map_err(|e| {
        eprintln!("[murmur:wayland] wl_register_all FAILED: {}", e);
        format!("wl_register_all failed: {}", e)
    })?;

    eprintln!(
        "[murmur:wayland] wl_register_all OK; portal binding established for {}",
        APP_ID
    );

    spawn_event_pump(app.clone());
    spawn_changed_pump(app.clone(), &manager);

    let _ = MANAGER.set(manager);
    Ok(())
}

fn spawn_event_pump(app: AppHandle) {
    std::thread::spawn(move || {
        let rx = GlobalHotKeyEvent::receiver();
        for event in rx.iter() {
            handle_event(&app, event);
        }
        log::warn!("Wayland hotkey event channel closed");
    });
}

fn spawn_changed_pump(app: AppHandle, manager: &GlobalHotKeyManager) {
    let Some(rx) = WlHotKeysChangedEvent::receiver() else {
        return;
    };
    let descriptions: Vec<String> = manager
        .wl_get_hotkeys()
        .iter()
        .map(|hk| format!("{}={}", hk.id(), hk.hotkey_description()))
        .collect();
    if !descriptions.is_empty() {
        log::info!("Wayland hotkey bindings: {}", descriptions.join(", "));
    }

    std::thread::spawn(move || {
        for change in rx.iter() {
            for ch in &change.changed_hotkeys {
                log::info!(
                    "User rebound Wayland action id={}: {}",
                    ch.id,
                    ch.hotkey_description
                );
                if ch.id == PTT_ACTION_ID {
                    if let Ok(store) = app.store("settings.json") {
                        store.set(
                            "hotkey",
                            serde_json::Value::String(ch.hotkey_description.clone()),
                        );
                        let _ = store.save();
                    }
                }
            }
        }
    });
}

fn handle_event(app: &AppHandle, event: GlobalHotKeyEvent) {
    let id = event.id();
    match id {
        PTT_ACTION_ID => handle_ptt(app, event.state()),
        SETTINGS_ACTION_ID => {
            if event.state() == HotKeyState::Pressed {
                log::debug!("Settings shortcut pressed -- opening settings");
                settings::open_settings_internal(app);
            }
        }
        other => log::debug!("Ignoring Wayland hotkey event for unknown id {}", other),
    }
}

fn handle_ptt(app: &AppHandle, state: HotKeyState) {
    let mode = get_record_mode(app);
    match state {
        HotKeyState::Pressed => {
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
        HotKeyState::Released => {
            if mode != "tap" {
                log::debug!("Hotkey released -- stopping recording");
                let _ = audio::stop_recording_internal(app.clone());
            }
        }
    }
}

fn get_record_mode(app: &AppHandle) -> String {
    app.store("settings.json")
        .ok()
        .and_then(|store| store.get("recordMode"))
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "hold".to_string())
}

fn is_recording(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .lock()
        .map(|inner| inner.recording_state == RecordingState::Recording)
        .unwrap_or(false)
}
