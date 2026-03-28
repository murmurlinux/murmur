use arboard::Clipboard;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use super::display_server::{self, DisplayServer};

/// Stores the last known non-Murmur window ID (X11 only).
static LAST_EXTERNAL_WINDOW: Mutex<Option<String>> = Mutex::new(None);

/// Stop flag for the window tracker thread.
static TRACKER_STOP: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Tool availability checks
// ---------------------------------------------------------------------------

/// Check if xdotool is available on the system.
pub fn is_xdotool_available() -> bool {
    Command::new("xdotool")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if wtype is available on the system.
fn is_wtype_available() -> bool {
    Command::new("wtype")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// X11 window tracking (not available on Wayland)
// ---------------------------------------------------------------------------

/// Get the currently focused window ID via xdotool.
pub fn get_active_window() -> Option<String> {
    Command::new("xdotool")
        .args(["getactivewindow"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

/// Start a background thread that tracks the last non-Murmur focused window.
/// Only runs on X11 — Wayland doesn't expose window IDs.
pub fn start_window_tracker() {
    let ds = display_server::detect();

    if ds == DisplayServer::Wayland {
        log::info!("Wayland detected — window tracking not available (by design).");
        return;
    }

    if !is_xdotool_available() {
        log::warn!("xdotool not found. Text injection will use clipboard only.");
        return;
    }

    TRACKER_STOP.store(false, Ordering::Relaxed);

    std::thread::spawn(|| {
        // Wait for Murmur's window to be created
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Get Murmur's window IDs using WM_CLASS
        let murmur_ids: Vec<String> = Command::new("xdotool")
            .args(["search", "--class", "murmur"])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .lines()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        log::debug!("Murmur window IDs (by class): {:?}", murmur_ids);

        while !TRACKER_STOP.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(200));

            if let Some(active) = get_active_window() {
                if !murmur_ids.contains(&active) {
                    if let Ok(mut last) = LAST_EXTERNAL_WINDOW.lock() {
                        *last = Some(active);
                    }
                }
            }
        }

        log::debug!("Window tracker thread stopped");
    });
}

/// Get the last known non-Murmur window ID (X11 only).
pub fn get_last_external_window() -> Option<String> {
    LAST_EXTERNAL_WINDOW.lock().ok().and_then(|w| w.clone())
}

// ---------------------------------------------------------------------------
// Text sanitisation
// ---------------------------------------------------------------------------

/// Sanitise transcribed text for safe injection.
/// Strips control characters that could produce unintended keystrokes.
fn sanitise_for_injection(text: &str) -> String {
    text.chars()
        .filter(|c| {
            // Allow printable ASCII, newline, tab, and all Unicode above ASCII
            matches!(*c, '\n' | '\t' | ' '..='~') || (*c as u32 > 127 && !c.is_control())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// X11 injection (xdotool — existing approach)
// ---------------------------------------------------------------------------

/// Inject text on X11 via xdotool XTEST.
fn paste_text_x11(text: &str, target_window: Option<&str>) -> Result<(), anyhow::Error> {
    let window_id = target_window
        .map(|s| s.to_string())
        .or_else(get_last_external_window);

    // Refocus the target window
    if let Some(ref wid) = window_id {
        let activate_status = Command::new("xdotool")
            .args(["windowactivate", "--sync", wid])
            .status();

        match activate_status {
            Ok(s) if s.success() => {
                log::debug!("Refocused window: {}", wid);
            }
            _ => {
                log::warn!("Failed to refocus window {}. Text is on clipboard.", wid);
                return Ok(());
            }
        }

        // Small delay for window manager to settle
        std::thread::sleep(std::time::Duration::from_millis(50));
    } else {
        log::warn!("No target window found. Text is on clipboard.");
        return Ok(());
    }

    // Type text using XTEST (no --window flag = universal compatibility)
    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", text])
        .status();

    match status {
        Ok(s) if s.success() => {
            log::debug!("xdotool typed: {:?}", &text[..text.len().min(50)]);
            Ok(())
        }
        Ok(s) => {
            log::warn!("xdotool type exited with: {}. Text is on clipboard.", s);
            Ok(())
        }
        Err(e) => {
            log::error!("xdotool failed: {}. Text is on clipboard.", e);
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Wayland injection (wtype + clipboard fallback)
// ---------------------------------------------------------------------------

/// Inject text on Wayland via wtype, falling back to clipboard + Ctrl+V.
fn paste_text_wayland(text: &str) -> Result<(), anyhow::Error> {
    // Try wtype first — Wayland-native, supports Unicode/CJK
    if is_wtype_available() {
        let status = Command::new("wtype").arg("--").arg(text).status();

        match status {
            Ok(s) if s.success() => {
                log::debug!("wtype typed: {:?}", &text[..text.len().min(50)]);
                return Ok(());
            }
            Ok(s) => {
                log::warn!("wtype exited with: {}. Trying Ctrl+V fallback.", s);
            }
            Err(e) => {
                log::warn!("wtype failed: {}. Trying Ctrl+V fallback.", e);
            }
        }
    } else {
        log::info!("wtype not found. Using clipboard + Ctrl+V fallback.");
    }

    // Fallback: clipboard is already set by caller, simulate Ctrl+V
    paste_via_ctrl_v_wayland()
}

/// Simulate Ctrl+V on Wayland via wtype key simulation.
fn paste_via_ctrl_v_wayland() -> Result<(), anyhow::Error> {
    // wtype can simulate key combos: -M = modifier down, -m = modifier up, -P/-p = key
    let status = Command::new("wtype")
        .args(["-M", "ctrl", "-P", "v", "-p", "v", "-m", "ctrl"])
        .status();

    match status {
        Ok(s) if s.success() => {
            log::debug!("Ctrl+V simulated via wtype");
            Ok(())
        }
        _ => {
            log::warn!("Ctrl+V simulation failed. Text is on clipboard — paste manually.");
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point — dispatches based on display server
// ---------------------------------------------------------------------------

/// Inject transcribed text into the target application.
///
/// Strategy:
///   - X11: refocus target window via xdotool, type via XTEST
///   - Wayland: type via wtype (Unicode-safe), fallback to clipboard + Ctrl+V
///   - Unknown: clipboard only
///
/// Text is always copied to clipboard as a universal fallback.
pub fn paste_text(text: &str, target_window: Option<&str>) -> Result<(), anyhow::Error> {
    if text.is_empty() {
        return Ok(());
    }

    let text = sanitise_for_injection(text);
    if text.is_empty() {
        return Ok(());
    }
    let text = text.as_str();

    // Always copy to clipboard as universal fallback
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text);
    }

    let ds = display_server::detect();
    log::debug!("Injecting text via {} display server", ds);

    match ds {
        DisplayServer::X11 => paste_text_x11(text, target_window),
        DisplayServer::Wayland => paste_text_wayland(text),
        DisplayServer::Unknown => {
            log::warn!("Unknown display server. Text is on clipboard — paste manually.");
            Ok(())
        }
    }
}
