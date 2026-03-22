# Murmur Production Readiness Audit

**Date:** 2026-03-22
**Team:** 5 specialist reviewers (security, Rust quality, frontend quality, packaging/UX, performance) + curator synthesis
**Scope:** Full codebase review of Murmur v0.1.0 — Tauri 2 + SolidJS + Rust + whisper.cpp desktop voice-to-text gadget

---

## Critical (must fix before release)

- **[C1] xdotool command injection via transcribed text** — `src-tauri/src/inject/paste.rs:120-122` — Whisper-transcribed text is passed directly as an argument to `xdotool type --clearmodifiers --delay 0 <text>`. While `Command::new().args()` does not go through a shell (so shell metacharacters like `; rm -rf /` are safe), xdotool itself interprets special key sequences in `type` mode. Characters like `\n`, `\t`, and certain escape sequences could produce unintended keystrokes. More critically, if whisper hallucinates or the audio is adversarial, the typed output could include unexpected content injected into any focused application (terminal, IDE, chat). **Recommendation:** Sanitise transcribed text before passing to xdotool — strip or escape control characters. Consider using clipboard paste (Ctrl+V via xdotool) as the primary injection method instead of `type`, which gives the user more control.

- **[C2] WhisperContext reloaded from disk on every transcription** — `src-tauri/src/stt/whisper.rs:31` — `WhisperContext::new_with_params()` is called on every single transcription. For the tiny model (~75MB), this means loading and parsing the model file from disk each time. This adds significant latency (hundreds of ms to seconds depending on disk) to every transcription and is the single largest performance bottleneck. **Recommendation:** Cache the `WhisperContext` in `AppState` (or a `OnceCell`/`OnceLock`), keyed by model path. Reload only when the user changes the active model.

- **[C3] No model file integrity verification** — `src-tauri/src/stt/model_manager.rs:118-153` — Model files are downloaded over HTTPS from Hugging Face with no checksum verification. If the download is interrupted, a partial/corrupt file is written and `dest.exists()` will return true on subsequent calls, causing whisper to load garbage. There is also no protection against MITM attacks on the HTTPS connection (though TLS provides baseline protection). **Recommendation:** (a) Write to a `.tmp` file and rename atomically on completion. (b) Verify SHA256 checksum of downloaded files against known-good hashes embedded in the `MODELS` constant. (c) Delete partial files on download failure.

- **[C4] Panic-inducing `.unwrap()` calls on fallible operations** — `src-tauri/src/commands/audio.rs:140,279` and `src-tauri/src/lib.rs:79` — `tokio::runtime::Runtime::new().unwrap()` is called inside transcription threads when a model needs downloading. If tokio runtime creation fails (e.g., OS thread limit reached), the app panics, leaving `recording_state` stuck in `Processing` permanently. Similarly, `app.default_window_icon().unwrap()` at `lib.rs:79` will crash the entire app on startup if the icon is misconfigured. **Recommendation:** Replace both with proper error handling — `Runtime::new().map_err(...)` with early return, and `.ok_or("No default icon")?` for the tray icon.

## Important (should fix)

- **[I1] Nested Mutex lock pattern — fragile deadlock potential** — `src-tauri/src/commands/audio.rs:40+55`, `audio.rs:206+216`, `audio.rs:83+94` — The outer `AppState` (`Mutex<InnerState>`) is locked, then `inner.audio_buffer.lock()` is called while still holding the outer lock. The audio capture thread independently locks `audio_buffer` at `capture.rs:174`. Currently safe because the capture thread never locks `AppState`, but this nested-lock pattern is a maintenance hazard — any future code that acquires these locks in reversed order will deadlock. **Recommendation:** Clone the `Arc<Mutex<Vec<f32>>>` from inner and drop the outer lock BEFORE locking the audio buffer.

- **[I2] Unbounded audio buffer growth** — `src-tauri/src/audio/capture.rs:174-176` — The `audio_buffer` (`Vec<f32>`) in `AppState` grows indefinitely during recording. At 44.1kHz mono, that is ~340KB/s. A 5-minute recording consumes ~100MB. There is no cap or ring buffer. **Recommendation:** Either cap at a maximum duration (e.g., 60s → ~10MB) and stop recording automatically, or use a ring buffer with a fixed ceiling.

- **[I3] Window tracker thread never terminates** — `src-tauri/src/inject/paste.rs:27-60` — The `start_window_tracker()` spawns a thread with an infinite `loop` polling `xdotool` every 200ms. This thread has no stop mechanism and runs for the lifetime of the process, spawning a new `xdotool` subprocess every 200ms (5/sec). On systems where `xdotool` is absent, each call fails silently, wasting resources. **Recommendation:** Add a stop flag (like the audio capture thread uses) and check for `xdotool` availability once at startup.

- **[I4] 50ms cursor polling loop with 3 IPC round-trips** — `src/components/GadgetWindow.tsx:61-89` — The click-through detection polls `cursorPosition()`, `outerPosition()`, and `innerSize()` every 50ms (20Hz). Each call is an async IPC round-trip to the Rust backend. That is 60 IPC calls/second when idle. **Recommendation:** Reduce poll frequency to 100-200ms (the cursor does not move that fast relative to the window), or move the hit-test logic entirely to the Rust side behind a single IPC command.

- **[I5] `requestAnimationFrame` loop runs continuously** — `src/components/Waveform.tsx:105` — The `draw()` function schedules itself via `requestAnimationFrame` unconditionally, even when no audio is recording. This causes continuous repainting (~60fps) of a canvas that shows a static idle line. **Recommendation:** Only run the rAF loop when `isActive()` is true, and render a single idle frame when transitioning to idle.

- **[I6] No LICENSE file** — Project root — The repository has no LICENSE file. This is a hard blocker for open-source distribution (all rights reserved by default). Even for proprietary distribution, a license should be declared. **Recommendation:** Add a LICENSE file (MIT, Apache 2.0, or proprietary — per your preference) before any public release.

- **[I7] Missing `as any` type safety erosion** — `src/components/GadgetWindow.tsx:21,152,153,169` — Multiple `as any` casts suppress TypeScript checking on skin zone types. If a skin config has a missing or malformed zone, this will crash at runtime rather than being caught at compile time. **Recommendation:** Define proper TypeScript interfaces for all zone types and validate skin configs at load time.

- **[I8] Duplicate `start_recording` / `stop_recording` logic** — `src-tauri/src/commands/audio.rs:36-76` vs `204-230`, and `78-201` vs `233-310` — The IPC-callable and internal (hotkey-callable) variants are near-identical ~80-line functions. Bugs fixed in one can easily be missed in the other. **Recommendation:** Extract a shared core function that takes `AppHandle` and have both variants delegate to it.

- **[I9] Hardcoded sample rate assumption** — `src-tauri/src/state.rs:27` — `sample_rate` defaults to 44100 but is never updated from the actual device config. If the default input device uses a different sample rate (48kHz is common), the resampling to 16kHz will use the wrong ratio, degrading transcription quality. **Recommendation:** Read the actual sample rate from `default_input_config()` in `capture.rs` and write it back to state before recording begins.

- **[I10] `println!` debug logging in production** — Multiple files (`paste.rs:46,105,108`, `capture.rs:50-55`, `audio.rs:74,110,138`, `model_manager.rs:109,116,152`, `hotkey.rs:25`, `whisper.rs:63-68`) — The codebase uses `println!`/`eprintln!` extensively for logging. These go to stdout/stderr which users will never see in a desktop app. **Recommendation:** Replace with a proper logging framework (`tracing` or `log` crate) with configurable log levels. This also enables diagnostic log collection for user bug reports.

## Nice-to-have (can defer)

- **[N1] Duplicate hue-to-hex conversion logic** — `src/components/GadgetWindow.tsx:108-125` and `src/components/SettingsPanel.tsx:73-104` — The hex-to-hue conversion is implemented independently in two components (GadgetWindow computes `hueRotation`, SettingsPanel has `hexToHue`/`hueToHex`). These should be consolidated into `src/lib/color.ts`. **Recommendation:** Extract to shared utility.

- **[N2] `WaveformPlaceholder.tsx` appears to be dead code** — `src/components/WaveformPlaceholder.tsx` — This file exists alongside the real `Waveform.tsx` and does not appear to be imported anywhere. **Recommendation:** Verify and remove if unused.

- **[N3] Settings window lacks error feedback** — `src/components/SettingsPanel.tsx` — Model download failures, hotkey registration failures, and store errors are caught but only logged to console. The user gets no visible feedback. **Recommendation:** Add toast notifications or inline error messages for user-facing operations.

- **[N4] No AppImage-specific runtime dependency handling** — `src-tauri/tauri.conf.json:44-47` — The `.deb` package correctly lists `xdotool` as a dependency, but AppImage users get no guidance. If `xdotool` is missing, the app silently falls back to clipboard-only mode. **Recommendation:** Detect `xdotool` absence at startup and show a one-time notification to the user.

- **[N5] Missing icon sizes** — `src-tauri/icons/` — Only 32, 128, 256, 512, and generic `icon.png` are present. Some Linux desktop environments expect 16x16, 48x48, or 64x64 icons. **Recommendation:** Generate additional standard icon sizes for broader desktop environment support.

- **[N6] `ort` crate pinned to release candidate** — `src-tauri/Cargo.toml:20` — `ort = "=2.0.0-rc.12"` is pinned to a pre-release version. Release candidates may contain breaking changes or bugs. **Recommendation:** Monitor for a stable 2.0.0 release and upgrade when available. Check if `ort` is actually used (it may be a leftover from before whisper-rs was adopted).

- **[N7] Tray "Always on Top" label does not update** — `src-tauri/src/lib.rs:60-64` — The tray menu item reads `"Always on Top  [ON]"` as a static string. Toggling always-on-top via the tray does not update this label. **Recommendation:** Update the menu item label when the state changes, or remove the `[ON]` suffix.

- **[N8] Linear interpolation resampling** — `src-tauri/src/stt/whisper.rs:4-20` — Audio resampling uses linear interpolation, which introduces aliasing artifacts. For speech-to-text this is likely acceptable (whisper is robust), but higher quality resampling (e.g., sinc interpolation via the `rubato` crate) would produce cleaner audio. **Recommendation:** Defer unless transcription quality issues arise.

## Specialist Disagreements

- **xdotool injection severity:** The security auditor rated this as Critical due to arbitrary keystroke injection potential. The Rust reviewer noted that `Command::new().args()` prevents shell injection. Both are correct — there is no *shell* injection, but xdotool's own `type` command can still produce unintended keystrokes from control characters in transcribed text. **Resolution:** Rated Critical (C1) because the attack surface exists even without shell involvement.

- **Cursor polling overhead:** The performance reviewer flagged 60 IPC calls/second as significant overhead. The frontend reviewer considered 50ms polling acceptable for responsive click-through UX. **Resolution:** Rated Important (I4). The UX concern is valid but the frequency can be reduced (100-200ms) without perceptible degradation, since the window is stationary most of the time.

- **rAF loop when idle:** The performance reviewer flagged continuous `requestAnimationFrame` as wasteful. The frontend reviewer noted the idle decay animation needs it briefly after recording stops. **Resolution:** Rated Important (I5). The decay animation needs ~500ms of rAF after going idle, not continuous. Add a timeout to stop the loop after decay completes.

- **Audio buffer growth severity:** The performance reviewer rated unbounded audio buffer as Critical (OOM risk). The Rust reviewer rated the buffer clone as Important. **Resolution:** Rated Important (I2). OOM requires unrealistically long recordings (1+ hours); typical voice dictation is seconds to minutes. The clone-instead-of-drain is a valid memory optimization but not a crash risk for normal use.

## Summary Verdict

Murmur is **functional and well-structured for a v0.1.0**, with clean separation of concerns (audio capture, STT, text injection, UI) and solid SolidJS/Tauri architecture. The codebase is readable and the core flow works end-to-end.

**However, 4 critical issues must be resolved before any public release:**

1. **C1 (xdotool text sanitisation)** — Security risk: unsanitised text injection
2. **C2 (whisper model caching)** — Performance: model reloaded every transcription
3. **C3 (download integrity)** — Reliability: corrupt partial downloads persist silently
4. **C4 (tokio runtime)** — Stability: runtime creation can panic

**Recommended next steps:**
1. Fix all 4 Critical items (estimated: 1-2 sessions)
2. Add LICENSE file (I6) — required for any distribution
3. Fix hardcoded sample rate (I9) — affects transcription quality on common hardware
4. Refactor duplicate recording logic (I8) — reduces maintenance risk
5. Address Important items I1-I5 (resource management and performance)
6. Nice-to-have items can be deferred to post-release polish

**Overall: Not yet production-ready, but close. The critical items are well-scoped fixes, not architectural problems.**
