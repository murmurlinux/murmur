//! Wayland text injection via xdg-desktop-portal RemoteDesktop.
//!
//! On GNOME 49 / KDE Plasma 6 the `zwp_virtual_keyboard_manager_v1`
//! protocol that `wtype` uses is gated to IME-class clients only. The
//! compositor maintainers explicitly steer everyone toward this portal
//! for synthetic input. The first call shows a one-time consent dialog;
//! the portal returns a `restore_token` that we cache to disk so all
//! subsequent calls are silent.
//!
//! This module is intentionally minimal: a single fresh portal session
//! per call, ASCII + Unicode keysyms only, no pointer / touch / clipboard
//! integration beyond the keyboard. Optimisations (long-lived sessions,
//! per-compositor priority, ydotool fallback) are tracked in
//! `murmurlinux/internal#138` and `#139`.

use std::path::PathBuf;
use std::time::Duration;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
use ashpd::desktop::PersistMode;

const RESTORE_TOKEN_FILENAME: &str = "portal-input-restore-token";
const PER_KEY_DELAY: Duration = Duration::from_millis(2);

/// Type `text` into the focused application via the RemoteDesktop portal.
///
/// First call surfaces a consent dialog; subsequent calls re-use the
/// cached `restore_token` and complete silently. Returns an error if the
/// portal is unavailable, the user denied consent, or any keysym fails;
/// callers should fall back to the clipboard path.
pub fn type_text(text: &str) -> Result<(), anyhow::Error> {
    if text.is_empty() {
        return Ok(());
    }

    // The transcription pipeline calls us from a plain `std::thread::spawn`
    // (no surrounding tokio runtime), so we can spin up a fresh one here.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(type_text_async(text))
}

async fn type_text_async(text: &str) -> Result<(), anyhow::Error> {
    let proxy = RemoteDesktop::new().await?;
    let session = proxy.create_session(Default::default()).await?;

    let cached_token = load_restore_token();
    let mut select_options = SelectDevicesOptions::default()
        .set_devices(Some(DeviceType::Keyboard.into()))
        .set_persist_mode(Some(PersistMode::Application));
    if let Some(token) = cached_token.as_deref() {
        select_options = select_options.set_restore_token(Some(token));
    }
    proxy.select_devices(&session, select_options).await?;

    let response = proxy
        .start(&session, None, Default::default())
        .await?
        .response()?;

    if let Some(token) = response.restore_token() {
        if cached_token.as_deref() != Some(token) {
            if let Err(e) = save_restore_token(token) {
                log::warn!(
                    "portal: failed to persist restore_token ({}); next launch will re-prompt",
                    e
                );
            }
        }
    }

    for c in text.chars() {
        let keysym = char_to_keysym(c);
        proxy
            .notify_keyboard_keysym(&session, keysym, KeyState::Pressed, Default::default())
            .await?;
        proxy
            .notify_keyboard_keysym(&session, keysym, KeyState::Released, Default::default())
            .await?;
        tokio::time::sleep(PER_KEY_DELAY).await;
    }

    Ok(())
}

/// Map a Unicode `char` to an X11 keysym. ASCII passes through verbatim
/// (the X11 ASCII keysym range matches the ASCII codepoint range).
/// Anything above 0x7F uses the X11 Unicode-keysym convention
/// (`0x01000000 | codepoint`). Newline and tab are mapped to their
/// dedicated keysyms.
fn char_to_keysym(c: char) -> i32 {
    match c {
        '\n' => 0xFF0D,     // XK_Return
        '\t' => 0xFF09,     // XK_Tab
        '\u{08}' => 0xFF08, // XK_BackSpace
        c if (c as u32) < 0x80 => c as i32,
        c => (0x01000000 | c as u32) as i32,
    }
}

fn restore_token_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "murmurlinux", "murmur")?;
    Some(dirs.config_dir().join(RESTORE_TOKEN_FILENAME))
}

fn load_restore_token() -> Option<String> {
    let path = restore_token_path()?;
    let contents = std::fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn save_restore_token(token: &str) -> std::io::Result<()> {
    let path = restore_token_path()
        .ok_or_else(|| std::io::Error::other("could not resolve project config dir"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, token)
}
