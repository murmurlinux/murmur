# Changelog

All notable changes to Murmur will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- **Slice 1** — Transparent skinned gadget window with interactive zones (mic button, gear button, status LEDs)
- **Slice 2** — Audio capture pipeline with live waveform visualisation via cpal (PipeWire/PulseAudio)
- **Slice 3** — Whisper transcription via whisper.cpp, text injection via xdotool, global hotkey (Ctrl+Shift+Space)
- **Slice 4** — Settings panel (hotkey, model, accent colour), click-through transparency, .deb + .AppImage packaging
- **Security audit** — 22 findings identified and resolved

### Technical Details

- Tauri 2 + SolidJS + Rust
- whisper.cpp via whisper-rs for local STT
- cpal for cross-audio-server capture
- xdotool (XTEST) for universal text injection
- Configurable global hotkey via tauri-plugin-global-shortcut
- Persistent settings via tauri-plugin-store
- Gemini v1 skin with accent colour hue rotation
