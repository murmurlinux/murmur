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

/// Construct a uinput virtual keyboard with just the keys we need to
/// synthesise Ctrl+V. Keeping the key set minimal narrows the helper's
/// blast radius if compromised.
fn build_virtual_keyboard() -> io::Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    keys.insert(KeyCode::KEY_LEFTCTRL);
    keys.insert(KeyCode::KEY_LEFTSHIFT);
    keys.insert(KeyCode::KEY_V);

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
///   `paste`         emit Ctrl+V on the virtual keyboard
///   `paste-shift`   emit Ctrl+Shift+V (terminal-friendly variant)
fn stdin_command_loop(uinput: Arc<Mutex<VirtualDevice>>) {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => return,
        };
        let cmd = line.trim();
        match cmd {
            "paste" => {
                if let Err(e) = emit_ctrl_v(&uinput, false) {
                    eprintln!("murmur-input-helper: paste failed: {}", e);
                }
            }
            "paste-shift" => {
                if let Err(e) = emit_ctrl_v(&uinput, true) {
                    eprintln!("murmur-input-helper: paste-shift failed: {}", e);
                }
            }
            "" => continue,
            other => eprintln!("murmur-input-helper: unknown command {:?}", other),
        }
    }
}

fn emit_ctrl_v(uinput: &Mutex<VirtualDevice>, with_shift: bool) -> io::Result<()> {
    let mut dev = uinput
        .lock()
        .map_err(|_| io::Error::other("uinput mutex poisoned"))?;

    let key_type = EventType::KEY.0;
    let mut sequence: Vec<InputEvent> = Vec::with_capacity(8);
    sequence.push(InputEvent::new(key_type, KeyCode::KEY_LEFTCTRL.0, 1));
    if with_shift {
        sequence.push(InputEvent::new(key_type, KeyCode::KEY_LEFTSHIFT.0, 1));
    }
    sequence.push(InputEvent::new(key_type, KeyCode::KEY_V.0, 1));
    sequence.push(InputEvent::new(key_type, KeyCode::KEY_V.0, 0));
    if with_shift {
        sequence.push(InputEvent::new(key_type, KeyCode::KEY_LEFTSHIFT.0, 0));
    }
    sequence.push(InputEvent::new(key_type, KeyCode::KEY_LEFTCTRL.0, 0));

    dev.emit(&sequence)?;
    // Tiny breath so the receiving app processes the chord before we
    // tear down the session.
    thread::sleep(Duration::from_millis(5));
    Ok(())
}
