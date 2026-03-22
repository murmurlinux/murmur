use arboard::Clipboard;
use std::process::Command;
use std::sync::Mutex;

/// Stores the last known non-Murmur window ID.
/// Updated by a background polling thread.
static LAST_EXTERNAL_WINDOW: Mutex<Option<String>> = Mutex::new(None);

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
/// Takes Murmur's own X11 window ID to exclude from tracking.
pub fn start_window_tracker() {
    std::thread::spawn(|| {
        // Wait for Murmur's window to be created, then find its X11 ID
        // by looking for windows with the exact class "murmur" (Tauri sets this from the app name)
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Get Murmur's window ID using WM_CLASS which is unique to our app
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

        println!("Murmur window IDs (by class): {:?}", murmur_ids);

        loop {
            std::thread::sleep(std::time::Duration::from_millis(200));

            if let Some(active) = get_active_window() {
                // Only store if it's NOT one of Murmur's windows
                if !murmur_ids.contains(&active) {
                    if let Ok(mut last) = LAST_EXTERNAL_WINDOW.lock() {
                        *last = Some(active);
                    }
                }
            }
        }
    });
}

/// Get the last known non-Murmur window ID.
/// This is the window we should paste into.
pub fn get_last_external_window() -> Option<String> {
    LAST_EXTERNAL_WINDOW.lock().ok().and_then(|w| w.clone())
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
                println!("Refocused window: {}", wid);
            }
            _ => {
                eprintln!("Failed to refocus window {}. Text is on clipboard.", wid);
                return Ok(());
            }
        }

        // Small delay for window manager to fully settle
        std::thread::sleep(std::time::Duration::from_millis(50));
    } else {
        eprintln!("No target window found. Text is on clipboard.");
        return Ok(());
    }

    // Step 2: Type text using XTEST (no --window flag = universal compatibility)
    let status = Command::new("xdotool")
        .args(["type", "--clearmodifiers", "--delay", "0", text])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Typed text: {:?}", &text[..text.len().min(50)]);
            Ok(())
        }
        Ok(s) => {
            eprintln!("xdotool type exited with: {}. Text is on clipboard.", s);
            Ok(())
        }
        Err(e) => {
            eprintln!("xdotool not found: {}. Text is on clipboard — paste manually.", e);
            Err(anyhow::anyhow!(
                "xdotool not installed. Text copied to clipboard — paste manually."
            ))
        }
    }
}
