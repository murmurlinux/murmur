//! Privileged input helper. Shipped at /usr/bin/murmur-input-helper with
//! mode 02755, owner root:input. The setgid bit gives this binary an
//! effective gid of `input`, so it can:
//!
//!   - Open /dev/input/event* to read keyboard events for the global
//!     hotkey, even when the calling user is not in the input group.
//!   - Open /dev/uinput to synthesise Ctrl+V keystrokes after Murmur
//!     transcribes audio, so the transcript is pasted at the cursor on
//!     Wayland sessions where wtype is gated by the compositor.
//!
//! Lifecycle:
//!   1. Open all keyboard /dev/input/event* devices and /dev/uinput
//!      while the effective gid is still `input`. The kernel checks
//!      DAC at open(); after open() the egid drop is safe.
//!   2. setresgid back to the caller's real gid so the rest of the
//!      process runs unprivileged. Open fds keep working.
//!   3. Spawn one OS thread per evdev device. Each forwards EV_KEY
//!      events (filtering value=2 key-repeat) to a single channel.
//!   4. Spawn a stdin reader thread that accepts simple newline
//!      delimited text commands; on "paste" it emits Ctrl+V on the
//!      virtual keyboard.
//!   5. Main thread drains the keyboard event channel and writes
//!      fixed 6-byte frames to stdout: u16 LE keycode + i32 LE value.
//!      No length prefix; frames are fixed size.
//!
//! On parent death the kernel closes our stdin/stdout, our next read
//! or write returns EOF/EPIPE, and we exit cleanly.

use std::io::{self, BufRead, BufReader, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, EventSummary, EventType, InputEvent, KeyCode};
use nix::unistd::{getgid, setresgid, Gid};

fn main() {
    if let Err(err) = run() {
        eprintln!("murmur-input-helper: {}", err);
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    // Open evdev keyboards and the uinput device while egid is still
    // `input`. We tolerate uinput failure (some environments restrict it
    // even from the input group) and continue with read-only mode --
    // paste will silently no-op and Murmur falls back to "text on
    // clipboard, paste manually".
    let read_devices = open_keyboards()?;
    let virtual_keyboard = match build_virtual_keyboard() {
        Ok(dev) => Some(Arc::new(Mutex::new(dev))),
        Err(e) => {
            eprintln!(
                "murmur-input-helper: uinput unavailable ({}); paste will be clipboard-only",
                e
            );
            None
        }
    };

    if read_devices.is_empty() && virtual_keyboard.is_none() {
        return Err(io::Error::other(
            "no readable keyboards and no writable uinput; helper has nothing to do",
        ));
    }
    eprintln!(
        "murmur-input-helper: read={} write={}",
        read_devices.len(),
        if virtual_keyboard.is_some() {
            "uinput"
        } else {
            "(none)"
        }
    );

    // Drop privileges. fds opened above stay valid because the kernel
    // checks DAC at open(), not on subsequent read/write.
    let real_gid = Gid::from_raw(getgid().as_raw());
    setresgid(real_gid, real_gid, real_gid)
        .map_err(|e| io::Error::other(format!("setresgid: {}", e)))?;

    let (tx, rx) = mpsc::channel::<(u16, i32)>();
    for device in read_devices {
        let tx = tx.clone();
        thread::spawn(move || pump_device(device, tx));
    }
    drop(tx);

    if let Some(uinput) = virtual_keyboard.clone() {
        thread::spawn(move || stdin_command_loop(uinput));
    }

    let mut out = io::stdout().lock();
    while let Ok((code, value)) = rx.recv() {
        out.write_all(&code.to_le_bytes())?;
        out.write_all(&value.to_le_bytes())?;
        out.flush()?;
    }
    Ok(())
}

fn open_keyboards() -> io::Result<Vec<Device>> {
    let mut devices = Vec::new();
    for (path, device) in evdev::enumerate() {
        let supported = match device.supported_keys() {
            Some(keys) => keys,
            None => continue,
        };
        if supported.contains(KeyCode::KEY_A) {
            eprintln!(
                "murmur-input-helper: opened {} ({})",
                path.display(),
                device.name().unwrap_or("?")
            );
            devices.push(device);
        }
    }
    Ok(devices)
}

/// Construct a uinput virtual keyboard covering the keys we need to
/// type printable ASCII text plus a few control combinations. We
/// register the full A-Z / 0-9 / common-symbol range so the helper
/// can synthesise typed transcripts that work in terminals (which do
/// not honour synthetic Ctrl+V) and any other text input.
fn build_virtual_keyboard() -> io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    for k in [
        // Modifiers
        KeyCode::KEY_LEFTCTRL,
        KeyCode::KEY_LEFTSHIFT,
        // Whitespace + control
        KeyCode::KEY_SPACE,
        KeyCode::KEY_ENTER,
        KeyCode::KEY_TAB,
        // Punctuation / symbols on US QWERTY
        KeyCode::KEY_GRAVE,
        KeyCode::KEY_MINUS,
        KeyCode::KEY_EQUAL,
        KeyCode::KEY_LEFTBRACE,
        KeyCode::KEY_RIGHTBRACE,
        KeyCode::KEY_BACKSLASH,
        KeyCode::KEY_SEMICOLON,
        KeyCode::KEY_APOSTROPHE,
        KeyCode::KEY_COMMA,
        KeyCode::KEY_DOT,
        KeyCode::KEY_SLASH,
        // Digits
        KeyCode::KEY_0,
        KeyCode::KEY_1,
        KeyCode::KEY_2,
        KeyCode::KEY_3,
        KeyCode::KEY_4,
        KeyCode::KEY_5,
        KeyCode::KEY_6,
        KeyCode::KEY_7,
        KeyCode::KEY_8,
        KeyCode::KEY_9,
        // Letters
        KeyCode::KEY_A,
        KeyCode::KEY_B,
        KeyCode::KEY_C,
        KeyCode::KEY_D,
        KeyCode::KEY_E,
        KeyCode::KEY_F,
        KeyCode::KEY_G,
        KeyCode::KEY_H,
        KeyCode::KEY_I,
        KeyCode::KEY_J,
        KeyCode::KEY_K,
        KeyCode::KEY_L,
        KeyCode::KEY_M,
        KeyCode::KEY_N,
        KeyCode::KEY_O,
        KeyCode::KEY_P,
        KeyCode::KEY_Q,
        KeyCode::KEY_R,
        KeyCode::KEY_S,
        KeyCode::KEY_T,
        KeyCode::KEY_U,
        KeyCode::KEY_V,
        KeyCode::KEY_W,
        KeyCode::KEY_X,
        KeyCode::KEY_Y,
        KeyCode::KEY_Z,
    ] {
        keys.insert(k);
    }

    VirtualDevice::builder()?
        .name("murmur-virtual-keyboard")
        .with_keys(&keys)?
        .build()
}

fn pump_device(mut device: Device, tx: mpsc::Sender<(u16, i32)>) {
    loop {
        let events = match device.fetch_events() {
            Ok(events) => events,
            Err(e) => {
                eprintln!("murmur-input-helper: device read error: {}", e);
                return;
            }
        };
        for ev in events {
            if let EventSummary::Key(_, code, value) = ev.destructure() {
                if value == 2 {
                    continue;
                }
                if tx.send((code.0, value)).is_err() {
                    return;
                }
            }
        }
    }
}

/// Read newline-delimited commands from stdin. The set is intentionally
/// tiny -- we accept exactly the operations the parent needs, nothing
/// arbitrary, so a compromised parent can only ask us to do things from
/// a narrow allowlist.
///
/// Commands:
///   `type <text>`   type the rest of the line as keystrokes (US QWERTY)
fn stdin_command_loop(uinput: Arc<Mutex<VirtualDevice>>) {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => return,
        };
        if line.is_empty() {
            continue;
        }
        if let Some(text) = line.strip_prefix("type ") {
            if let Err(e) = emit_typed_text(&uinput, text) {
                eprintln!("murmur-input-helper: type failed: {}", e);
            }
        } else {
            eprintln!("murmur-input-helper: unknown command {:?}", line);
        }
    }
}

/// Translate `text` into press/release sequences on the virtual
/// keyboard, applying Shift where the US QWERTY mapping requires it.
/// Characters with no mapping are silently skipped -- callers can
/// inspect their original text against what arrived if they need a
/// completeness check.
fn emit_typed_text(uinput: &Mutex<VirtualDevice>, text: &str) -> io::Result<()> {
    let mut dev = uinput
        .lock()
        .map_err(|_| io::Error::other("uinput mutex poisoned"))?;

    let key_type = EventType::KEY.0;
    let shift_code = KeyCode::KEY_LEFTSHIFT.0;
    for c in text.chars() {
        let Some((key, needs_shift)) = char_to_key(c) else {
            continue;
        };
        let mut events: Vec<InputEvent> = Vec::with_capacity(4);
        if needs_shift {
            events.push(InputEvent::new(key_type, shift_code, 1));
        }
        events.push(InputEvent::new(key_type, key.0, 1));
        events.push(InputEvent::new(key_type, key.0, 0));
        if needs_shift {
            events.push(InputEvent::new(key_type, shift_code, 0));
        }
        dev.emit(&events)?;
    }
    // Tiny breath so the receiving app drains the queue before any
    // follow-up work happens on our side.
    thread::sleep(Duration::from_millis(5));
    Ok(())
}

/// Map a Unicode `char` to the keycode + shift state that produces it
/// on a US QWERTY layout. Returns `None` for chars we cannot type
/// (non-ASCII, or symbols outside the printable range).
#[allow(clippy::too_many_lines)]
fn char_to_key(c: char) -> Option<(KeyCode, bool)> {
    Some(match c {
        // Whitespace / control
        ' ' => (KeyCode::KEY_SPACE, false),
        '\n' => (KeyCode::KEY_ENTER, false),
        '\t' => (KeyCode::KEY_TAB, false),
        // Letters
        'a' => (KeyCode::KEY_A, false),
        'b' => (KeyCode::KEY_B, false),
        'c' => (KeyCode::KEY_C, false),
        'd' => (KeyCode::KEY_D, false),
        'e' => (KeyCode::KEY_E, false),
        'f' => (KeyCode::KEY_F, false),
        'g' => (KeyCode::KEY_G, false),
        'h' => (KeyCode::KEY_H, false),
        'i' => (KeyCode::KEY_I, false),
        'j' => (KeyCode::KEY_J, false),
        'k' => (KeyCode::KEY_K, false),
        'l' => (KeyCode::KEY_L, false),
        'm' => (KeyCode::KEY_M, false),
        'n' => (KeyCode::KEY_N, false),
        'o' => (KeyCode::KEY_O, false),
        'p' => (KeyCode::KEY_P, false),
        'q' => (KeyCode::KEY_Q, false),
        'r' => (KeyCode::KEY_R, false),
        's' => (KeyCode::KEY_S, false),
        't' => (KeyCode::KEY_T, false),
        'u' => (KeyCode::KEY_U, false),
        'v' => (KeyCode::KEY_V, false),
        'w' => (KeyCode::KEY_W, false),
        'x' => (KeyCode::KEY_X, false),
        'y' => (KeyCode::KEY_Y, false),
        'z' => (KeyCode::KEY_Z, false),
        'A' => (KeyCode::KEY_A, true),
        'B' => (KeyCode::KEY_B, true),
        'C' => (KeyCode::KEY_C, true),
        'D' => (KeyCode::KEY_D, true),
        'E' => (KeyCode::KEY_E, true),
        'F' => (KeyCode::KEY_F, true),
        'G' => (KeyCode::KEY_G, true),
        'H' => (KeyCode::KEY_H, true),
        'I' => (KeyCode::KEY_I, true),
        'J' => (KeyCode::KEY_J, true),
        'K' => (KeyCode::KEY_K, true),
        'L' => (KeyCode::KEY_L, true),
        'M' => (KeyCode::KEY_M, true),
        'N' => (KeyCode::KEY_N, true),
        'O' => (KeyCode::KEY_O, true),
        'P' => (KeyCode::KEY_P, true),
        'Q' => (KeyCode::KEY_Q, true),
        'R' => (KeyCode::KEY_R, true),
        'S' => (KeyCode::KEY_S, true),
        'T' => (KeyCode::KEY_T, true),
        'U' => (KeyCode::KEY_U, true),
        'V' => (KeyCode::KEY_V, true),
        'W' => (KeyCode::KEY_W, true),
        'X' => (KeyCode::KEY_X, true),
        'Y' => (KeyCode::KEY_Y, true),
        'Z' => (KeyCode::KEY_Z, true),
        // Digits
        '0' => (KeyCode::KEY_0, false),
        '1' => (KeyCode::KEY_1, false),
        '2' => (KeyCode::KEY_2, false),
        '3' => (KeyCode::KEY_3, false),
        '4' => (KeyCode::KEY_4, false),
        '5' => (KeyCode::KEY_5, false),
        '6' => (KeyCode::KEY_6, false),
        '7' => (KeyCode::KEY_7, false),
        '8' => (KeyCode::KEY_8, false),
        '9' => (KeyCode::KEY_9, false),
        // Shifted digits (US QWERTY)
        ')' => (KeyCode::KEY_0, true),
        '!' => (KeyCode::KEY_1, true),
        '@' => (KeyCode::KEY_2, true),
        '#' => (KeyCode::KEY_3, true),
        '$' => (KeyCode::KEY_4, true),
        '%' => (KeyCode::KEY_5, true),
        '^' => (KeyCode::KEY_6, true),
        '&' => (KeyCode::KEY_7, true),
        '*' => (KeyCode::KEY_8, true),
        '(' => (KeyCode::KEY_9, true),
        // Punctuation
        '`' => (KeyCode::KEY_GRAVE, false),
        '~' => (KeyCode::KEY_GRAVE, true),
        '-' => (KeyCode::KEY_MINUS, false),
        '_' => (KeyCode::KEY_MINUS, true),
        '=' => (KeyCode::KEY_EQUAL, false),
        '+' => (KeyCode::KEY_EQUAL, true),
        '[' => (KeyCode::KEY_LEFTBRACE, false),
        '{' => (KeyCode::KEY_LEFTBRACE, true),
        ']' => (KeyCode::KEY_RIGHTBRACE, false),
        '}' => (KeyCode::KEY_RIGHTBRACE, true),
        '\\' => (KeyCode::KEY_BACKSLASH, false),
        '|' => (KeyCode::KEY_BACKSLASH, true),
        ';' => (KeyCode::KEY_SEMICOLON, false),
        ':' => (KeyCode::KEY_SEMICOLON, true),
        '\'' => (KeyCode::KEY_APOSTROPHE, false),
        '"' => (KeyCode::KEY_APOSTROPHE, true),
        ',' => (KeyCode::KEY_COMMA, false),
        '<' => (KeyCode::KEY_COMMA, true),
        '.' => (KeyCode::KEY_DOT, false),
        '>' => (KeyCode::KEY_DOT, true),
        '/' => (KeyCode::KEY_SLASH, false),
        '?' => (KeyCode::KEY_SLASH, true),
        _ => return None,
    })
}
