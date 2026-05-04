//! Linux global hotkey via direct evdev reads.
//!
//! Used on Wayland sessions where the X11 `XGrabKey` path used by
//! `tauri-plugin-global-shortcut` cannot see hardware key events for
//! non-focused windows, and where the `org.freedesktop.portal.GlobalShortcuts`
//! API does not deliver reliable Released events on GNOME Mutter for
//! hold-to-talk semantics. Reads `EV_KEY` events directly from
//! `/dev/input/event*` and runs a per-chord state machine.
//!
//! Read access to input devices is granted by the udev rule
//! `/lib/udev/rules.d/99-murmur.rules` shipped in the .deb postinst,
//! which sets `TAG+="uaccess"` so systemd-logind grants the active
//! local user session-scoped ACL access. No `input` group membership
//! required on stock Ubuntu 26.04 LTS.

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock};

use evdev::KeyCode;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::commands::{audio, settings};
use crate::state::{AppState, RecordingState};

const PTT_ACTION_ID: u32 = 0;
const SETTINGS_ACTION_ID: u32 = 1;
const SETTINGS_HOTKEY: &str = "Ctrl+Shift+Comma";

/// One-shot guard: the chord listener threads outlive the call site, so
/// repeat calls (e.g. when the user changes their hotkey from settings)
/// must not spawn duplicate listeners. A future iteration will support
/// live rebinds without restart.
static STARTED: OnceLock<()> = OnceLock::new();

/// Writer end of the pipe to the privileged helper. Held for the helper's
/// lifetime so [`crate::inject::paste`] can ask the helper to synthesise
/// Ctrl+V on Wayland sessions where wtype is gated by the compositor.
static HELPER_STDIN: OnceLock<Mutex<ChildStdin>> = OnceLock::new();

/// Send a command line to the privileged helper. Returns `Ok(())` if the
/// helper is running and the line was written; `Err` if the helper was
/// never spawned (no Wayland fallback was needed) or the pipe is dead.
pub fn send_helper_command(line: &str) -> std::io::Result<()> {
    let stdin = HELPER_STDIN
        .get()
        .ok_or_else(|| std::io::Error::other("helper is not running"))?;
    let mut guard = stdin
        .lock()
        .map_err(|_| std::io::Error::other("helper stdin mutex poisoned"))?;
    guard.write_all(line.as_bytes())?;
    if !line.ends_with('\n') {
        guard.write_all(b"\n")?;
    }
    guard.flush()
}

/// A registered hotkey expressed as keycodes the kernel emits.
///
/// `modifier_groups` is an AND-of-OR set: every group must have at
/// least one keycode currently pressed for the chord to count as held.
/// `key` is the non-modifier keycap that completes the chord.
#[derive(Debug, Clone)]
struct Chord {
    id: u32,
    modifier_groups: Vec<Vec<KeyCode>>,
    key: KeyCode,
}

impl Chord {
    fn is_held(&self, pressed: &HashSet<KeyCode>) -> bool {
        if !pressed.contains(&self.key) {
            return false;
        }
        self.modifier_groups
            .iter()
            .all(|group| group.iter().any(|k| pressed.contains(k)))
    }
}

/// Register PTT + Settings chords once and start the device pumps.
pub fn register(app: &AppHandle, ptt_hotkey: &str) -> Result<(), String> {
    if STARTED.get().is_some() {
        eprintln!(
            "[murmur:evdev] hotkeys already registered; ignoring repeat call (preference '{}')",
            ptt_hotkey
        );
        return Ok(());
    }

    let chords = vec![
        parse_chord(PTT_ACTION_ID, ptt_hotkey)?,
        parse_chord(SETTINGS_ACTION_ID, SETTINGS_HOTKEY)?,
    ];
    eprintln!(
        "[murmur:evdev] parsed chords: ptt={:?} settings={:?}",
        chords[0], chords[1]
    );

    let (tx, rx) = std::sync::mpsc::channel::<(KeyCode, i32)>();

    // Always spawn the helper. It owns both the evdev read path and the
    // uinput write path used by the paste-after-transcribe injector. The
    // helper drops gid back to the caller's after opening the devices, so
    // only the brief setup window holds elevated privilege.
    spawn_helper(tx.clone())?;

    let app_clone = app.clone();
    std::thread::spawn(move || run_arbiter(app_clone, chords, rx));

    let _ = STARTED.set(());
    Ok(())
}

/// Locate the privileged input helper binary. Production install ships it
/// alongside the main binary at `/usr/bin/murmur-input-helper`, mirroring
/// the wireshark-common layout for `/usr/bin/dumpcap`. `MURMUR_INPUT_HELPER`
/// overrides for dev builds; falling back to a sibling of the running
/// binary is a last-resort dev convenience.
fn helper_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("MURMUR_INPUT_HELPER") {
        return Some(PathBuf::from(p));
    }
    let production = Path::new("/usr/bin/murmur-input-helper");
    if production.exists() {
        return Some(production.to_path_buf());
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("murmur-input-helper");
            if sibling.exists() {
                return Some(sibling);
            }
        }
    }
    None
}

fn spawn_helper(tx: std::sync::mpsc::Sender<(KeyCode, i32)>) -> Result<(), String> {
    let path = helper_path().ok_or_else(|| {
        "Cannot find murmur-input-helper. Reinstall Murmur, or set the \
         MURMUR_INPUT_HELPER env var to its path."
            .to_string()
    })?;
    eprintln!("[murmur:evdev] spawning helper: {}", path.display());

    let mut child = Command::new(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", path.display(), e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "helper produced no stdout pipe".to_string())?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "helper produced no stdin pipe".to_string())?;

    // Detach the child handle. The helper lives for the murmur process
    // lifetime; closing our pipes (when we exit) gives it EOF/EPIPE so
    // it tears down cleanly.
    std::mem::forget(child);

    let _ = HELPER_STDIN.set(Mutex::new(stdin));

    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut frame = [0u8; 6];
        loop {
            if reader.read_exact(&mut frame).is_err() {
                eprintln!("[murmur:evdev] helper stdout closed");
                return;
            }
            let code_raw = u16::from_le_bytes([frame[0], frame[1]]);
            let value = i32::from_le_bytes([frame[2], frame[3], frame[4], frame[5]]);
            let code = KeyCode::new(code_raw);
            if tx.send((code, value)).is_err() {
                return;
            }
        }
    });

    Ok(())
}

fn parse_chord(id: u32, s: &str) -> Result<Chord, String> {
    let mut modifier_groups: Vec<Vec<KeyCode>> = Vec::new();
    let mut key: Option<KeyCode> = None;

    for part in s.split('+').map(str::trim) {
        let lower = part.to_ascii_lowercase();
        let mods_or_key = match lower.as_str() {
            "ctrl" | "control" => Some(vec![KeyCode::KEY_LEFTCTRL, KeyCode::KEY_RIGHTCTRL]),
            "shift" => Some(vec![KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_RIGHTSHIFT]),
            "alt" | "option" => Some(vec![KeyCode::KEY_LEFTALT, KeyCode::KEY_RIGHTALT]),
            "super" | "meta" | "win" | "cmd" => {
                Some(vec![KeyCode::KEY_LEFTMETA, KeyCode::KEY_RIGHTMETA])
            }
            _ => None,
        };
        if let Some(mods) = mods_or_key {
            modifier_groups.push(mods);
            continue;
        }
        if key.is_some() {
            return Err(format!(
                "Chord '{}' specifies more than one non-modifier key",
                s
            ));
        }
        key = Some(parse_key(&lower).ok_or_else(|| format!("Unknown key '{}'", part))?);
    }

    let key = key.ok_or_else(|| format!("Chord '{}' has no main key", s))?;
    Ok(Chord {
        id,
        modifier_groups,
        key,
    })
}

fn parse_key(lower: &str) -> Option<KeyCode> {
    match lower {
        "space" => Some(KeyCode::KEY_SPACE),
        "comma" | "," => Some(KeyCode::KEY_COMMA),
        "period" | "dot" | "." => Some(KeyCode::KEY_DOT),
        "slash" | "/" => Some(KeyCode::KEY_SLASH),
        "tab" => Some(KeyCode::KEY_TAB),
        "enter" | "return" => Some(KeyCode::KEY_ENTER),
        "escape" | "esc" => Some(KeyCode::KEY_ESC),
        "backspace" => Some(KeyCode::KEY_BACKSPACE),
        "minus" | "-" => Some(KeyCode::KEY_MINUS),
        "equal" | "=" => Some(KeyCode::KEY_EQUAL),
        "semicolon" | ";" => Some(KeyCode::KEY_SEMICOLON),
        "apostrophe" | "'" => Some(KeyCode::KEY_APOSTROPHE),
        "grave" | "`" => Some(KeyCode::KEY_GRAVE),
        "leftbracket" | "[" => Some(KeyCode::KEY_LEFTBRACE),
        "rightbracket" | "]" => Some(KeyCode::KEY_RIGHTBRACE),
        "backslash" | "\\" => Some(KeyCode::KEY_BACKSLASH),
        "f1" => Some(KeyCode::KEY_F1),
        "f2" => Some(KeyCode::KEY_F2),
        "f3" => Some(KeyCode::KEY_F3),
        "f4" => Some(KeyCode::KEY_F4),
        "f5" => Some(KeyCode::KEY_F5),
        "f6" => Some(KeyCode::KEY_F6),
        "f7" => Some(KeyCode::KEY_F7),
        "f8" => Some(KeyCode::KEY_F8),
        "f9" => Some(KeyCode::KEY_F9),
        "f10" => Some(KeyCode::KEY_F10),
        "f11" => Some(KeyCode::KEY_F11),
        "f12" => Some(KeyCode::KEY_F12),
        s if s.len() == 1 => match s.chars().next().unwrap() {
            'a' => Some(KeyCode::KEY_A),
            'b' => Some(KeyCode::KEY_B),
            'c' => Some(KeyCode::KEY_C),
            'd' => Some(KeyCode::KEY_D),
            'e' => Some(KeyCode::KEY_E),
            'f' => Some(KeyCode::KEY_F),
            'g' => Some(KeyCode::KEY_G),
            'h' => Some(KeyCode::KEY_H),
            'i' => Some(KeyCode::KEY_I),
            'j' => Some(KeyCode::KEY_J),
            'k' => Some(KeyCode::KEY_K),
            'l' => Some(KeyCode::KEY_L),
            'm' => Some(KeyCode::KEY_M),
            'n' => Some(KeyCode::KEY_N),
            'o' => Some(KeyCode::KEY_O),
            'p' => Some(KeyCode::KEY_P),
            'q' => Some(KeyCode::KEY_Q),
            'r' => Some(KeyCode::KEY_R),
            's' => Some(KeyCode::KEY_S),
            't' => Some(KeyCode::KEY_T),
            'u' => Some(KeyCode::KEY_U),
            'v' => Some(KeyCode::KEY_V),
            'w' => Some(KeyCode::KEY_W),
            'x' => Some(KeyCode::KEY_X),
            'y' => Some(KeyCode::KEY_Y),
            'z' => Some(KeyCode::KEY_Z),
            '0' => Some(KeyCode::KEY_0),
            '1' => Some(KeyCode::KEY_1),
            '2' => Some(KeyCode::KEY_2),
            '3' => Some(KeyCode::KEY_3),
            '4' => Some(KeyCode::KEY_4),
            '5' => Some(KeyCode::KEY_5),
            '6' => Some(KeyCode::KEY_6),
            '7' => Some(KeyCode::KEY_7),
            '8' => Some(KeyCode::KEY_8),
            '9' => Some(KeyCode::KEY_9),
            _ => None,
        },
        _ => None,
    }
}

fn run_arbiter(app: AppHandle, chords: Vec<Chord>, rx: std::sync::mpsc::Receiver<(KeyCode, i32)>) {
    let mut pressed: HashSet<KeyCode> = HashSet::new();
    let mut chord_held: HashMap<u32, bool> = HashMap::new();

    while let Ok((code, value)) = rx.recv() {
        match value {
            0 => {
                pressed.remove(&code);
            }
            1 => {
                pressed.insert(code);
            }
            _ => continue,
        }
        for chord in &chords {
            let now = chord.is_held(&pressed);
            let was = chord_held.get(&chord.id).copied().unwrap_or(false);
            if now == was {
                continue;
            }
            chord_held.insert(chord.id, now);
            dispatch(&app, chord.id, now);
        }
    }
    eprintln!("[murmur:evdev] arbiter channel closed");
}

fn dispatch(app: &AppHandle, action_id: u32, pressed: bool) {
    eprintln!("[murmur:evdev] action_id={} pressed={}", action_id, pressed);
    match action_id {
        PTT_ACTION_ID => handle_ptt(app, pressed),
        SETTINGS_ACTION_ID if pressed => settings::open_settings_internal(app),
        _ => {}
    }
}

fn handle_ptt(app: &AppHandle, pressed: bool) {
    let mode = get_record_mode(app);
    if pressed {
        if mode == "tap" {
            if is_recording(app) {
                let _ = audio::stop_recording_internal(app.clone());
            } else {
                let _ = audio::start_recording_internal(app.clone());
            }
        } else {
            let _ = audio::start_recording_internal(app.clone());
        }
    } else if mode != "tap" {
        let _ = audio::stop_recording_internal(app.clone());
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
