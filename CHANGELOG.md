# Changelog

All notable changes to Murmur will be documented in this file.

## [0.1.1] - 2026-03-26

### Added

- **Tap-to-toggle recording**:new recording mode alongside hold-to-record. Tap hotkey or mic button once to start, again to stop. Configurable in settings.
- **Show/hide skin**:hide the Comm Badge to system tray via menu or setting. Persists across restarts. "Show skin on startup" toggle in settings.
- **Recording indicator popup**:floating teal pill with M logo and animated waveform bars. Appears at bottom-center of screen when recording with skin hidden. Accent colour follows user's colour picker.
- **Settings redesign**:ocean terminal theme with glass cards, teal accent labels, brand logo header. Native GNOME window with proper resize/snap behaviour.

### Fixed

- Default accent colour changed from cyan (#00d4ff) to brand green (#10b981) matching the website
- Settings window background matches theme (no white border flash)
- Native window decorations for proper GNOME desktop integration

---

## [0.1.0] - 2026-03-18

### Added

- **Slice 1**:Transparent skinned gadget window with interactive zones (mic button, gear button, status LEDs)
- **Slice 2**:Audio capture pipeline with live waveform visualisation via cpal (PipeWire/PulseAudio)
- **Slice 3**:Whisper transcription via whisper.cpp, text injection via xdotool, global hotkey (Ctrl+Shift+Space)
- **Slice 4**:Settings panel (hotkey, model, accent colour), click-through transparency, .deb + .AppImage packaging
- **Security audit**:22 findings identified and resolved

### Technical Details

- Tauri 2 + SolidJS + Rust
- whisper.cpp via whisper-rs for local STT
- cpal for cross-audio-server capture
- xdotool (XTEST) for universal text injection
- Configurable global hotkey via tauri-plugin-global-shortcut
- Persistent settings via tauri-plugin-store
- Comm Badge skin with accent colour hue rotation
