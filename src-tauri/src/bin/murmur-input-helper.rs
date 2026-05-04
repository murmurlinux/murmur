//! Privileged keyboard reader. Shipped at /usr/libexec/murmur/murmur-input-helper
//! with mode 02755, owner root:input. The setgid bit gives this binary an
//! effective gid of `input`, so it can open /dev/input/event* even when the
//! user invoking it is not in the input group.
//!
//! Lifecycle:
//!   1. Enumerate /dev/input/event* and open every device that looks like a
//!      keyboard. The kernel checks DAC at open(); the setgid `input` egid is
//!      what grants access. Opening happens before any privilege drop.
//!   2. setresgid back to the caller's real gid. The opened fds remain valid;
//!      the kernel does not re-check permissions on read().
//!   3. Spawn one OS thread per device. Each reads EV_KEY events, filters out
//!      key-repeat (value=2), and forwards (code, value) frames to a single
//!      writer.
//!   4. The writer emits fixed 6-byte frames to stdout: u16 LE keycode, then
//!      i32 LE value. No length prefix; frames are fixed size. Parent pipes
//!      stdout and reads frames in lockstep.
//!
//! On parent death the kernel closes our stdout, our next write returns
//! EPIPE, and we exit cleanly.

use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;

use evdev::{Device, EventSummary, KeyCode};
use nix::unistd::{getgid, setresgid, Gid};

fn main() {
    if let Err(err) = run() {
        eprintln!("murmur-input-helper: {}", err);
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let devices = open_keyboards()?;
    if devices.is_empty() {
        return Err(io::Error::other(
            "no readable keyboard devices in /dev/input",
        ));
    }
    eprintln!(
        "murmur-input-helper: opened {} keyboard device(s)",
        devices.len()
    );

    let real_gid = Gid::from_raw(getgid().as_raw());
    setresgid(real_gid, real_gid, real_gid)
        .map_err(|e| io::Error::other(format!("setresgid: {}", e)))?;

    let (tx, rx) = mpsc::channel::<(u16, i32)>();
    for device in devices {
        let tx = tx.clone();
        thread::spawn(move || pump_device(device, tx));
    }
    drop(tx);

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
