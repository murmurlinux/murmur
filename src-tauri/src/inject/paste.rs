use arboard::Clipboard;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Stores the last known non-Murmur window ID.
/// Updated by a background polling thread.
static LAST_EXTERNAL_WINDOW: Mutex<Option<String>> = Mutex::new(None);

/// Stop flag for the window tracker thread.
static TRACKER_STOP: AtomicBool = AtomicBool::new(false);

/// Check if xdotool is available on the system.
pub fn is_xdotool_available() -> bool {
    Command::new("xdotool")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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
/// Checks for xdotool availability before starting.
pub fn start_window_tracker() {
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

/// Stop the window tracker thread (called on app shutdown).
#[allow(dead_code)]
pub fn stop_window_tracker() {
    TRACKER_STOP.store(true, Ordering::Relaxed);
}

/// Get the last known non-Murmur window ID.
/// This is the window we should paste into.
pub fn get_last_external_window() -> Option<String> {
    LAST_EXTERNAL_WINDOW.lock().ok().and_then(|w| w.clone())
}

/// Sanitise transcribed text for safe xdotool injection.
/// Strips control characters that could produce unintended keystrokes
/// (escape sequences, carriage returns, etc.) in the target application.
fn sanitise_for_xdotool(text: &str) -> String {
    text.chars()
        .filter(|c| {
            // Allow printable ASCII, newline, tab, and all Unicode above ASCII
            matches!(*c, '\n' | '\t' | ' '..='~') || (*c as u32 > 127 && !c.is_control())
        })
        .collect()
}

/// Inject transcribed text into the last external window.
///
/// Strategy (from xdotool documentation):
///   1. Refocus the target window using `xdotool windowactivate --sync <id>`
///      (--sync waits until the window is actually active before proceeding)
///   2. Type text using `xdotool type` WITHOUT --window flag
///      (without --window, xdotool uses XTEST which works in ALL apps;
///       with --window it uses XSendEvent which many apps reject)
///   3. Also copy text to clipboard as fallback for manual paste
///
/// This approach works universally: terminals, text editors, browsers,
/// IDEs — any X11 application that accepts keyboard input.
pub fn paste_text(text: &str, target_window: Option<&str>) -> Result<(), anyhow::Error> {
    if text.is_empty() {
        return Ok(());
    }

    // Sanitise: strip control characters that could cause unintended keystrokes
    let text = sanitise_for_xdotool(text);
    if text.is_empty() {
        return Ok(());
    }
    let text = text.as_str();

    // Always put text on clipboard as fallback for manual paste
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(text);
    }

    // Determine which window to target
    let window_id = target_window
        .map(|s| s.to_string())
        .or_else(get_last_external_window);

    // Step 1: Refocus the target window
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

        // Small delay for window manager to fully settle
        std::thread::sleep(std::time::Duration::from_millis(50));
    } else {
        log::warn!("No target window found. Text is on clipboard.");
        return Ok(());
    }

    // Step 2: Type text using XTEST (no --window flag = universal compatibility)
    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", text])
        .status();

    match status {
        Ok(s) if s.success() => {
            log::debug!("Typed text: {:?}", &text[..text.len().min(50)]);
            Ok(())
        }
        Ok(s) => {
            log::warn!("xdotool type exited with: {}. Text is on clipboard.", s);
            Ok(())
        }
        Err(e) => {
            log::error!("xdotool not found: {}. Text is on clipboard — paste manually.", e);
            Err(anyhow::anyhow!(
                "xdotool not installed. Text copied to clipboard — paste manually."
            ))
        }
    }
}
