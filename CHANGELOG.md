# Changelog

All notable changes to Murmur will be documented in this file.

## [Unreleased]

### Changed

- Switched to a tray-only architecture. The floating desktop gadget has been replaced with a system tray icon; left-click opens Settings, right-click shows the menu.
- Rebrand: rust-on-cream palette with JetBrains Mono throughout. Settings panel, onboarding wizard, and recording popup all retheme to match.
- Audio capture now supports i16 and u16 sample formats (previously f32-only); stream init errors are propagated to the caller instead of being logged silently.
- Onboarding and settings windows use a cream background to eliminate the white / dark flash on open.

### Fixed

- Onboarding wizard no longer freezes when clicking Next on the model step.
- Model downloads retry on transient failures; small-model download error ("error decoding response body") resolved.
- Dependencies bumped to clear RUSTSEC advisories (rustls-webpki, rand).

### Removed

- Skin system (floating widget, custom skin images, accent-colour picker). Replaced by the tray-only model.

---

## [0.3.3] - 2026-04-03

### Added

- Keyboard shortcut to open settings (Ctrl+Shift+,)
- Dynamic system tray tooltip shows recording state (Recording, Processing, Idle)
- Blind property test suite: 32 Rust + 26 TypeScript tests in CI
- CLAUDE.md test blindness policy

### Changed

- Upgraded reqwest from 0.12 to 0.13 (eliminates duplicate dependency, switches TLS to rustls)
- Renamed Cargo package from `murmur` to `murmur_lib` for test crate compatibility
- MODELS registry converted from tuple array to ModelEntry struct with named fields
- Exposed public API for external test crates: resample, sanitise_for_injection, trim_trailing_silence, DisplayServer, MODELS

### Fixed

- All-silence audio no longer returned unchanged by trim_trailing_silence (caused Whisper hallucinations)
- Leading partial audio chunk now scanned for speech (short utterances no longer silently dropped)
- hueToHex normalises hue input to [0, 360) so hueToHex(360.1) matches hueToHex(0.1)
- Blind test CI steps skip on fork PRs where secrets are unavailable

---

## [0.3.2] - 2026-03-30

### Fixed

- Install instructions and binary size claims updated

---

## [0.3.1] - 2026-03-30

### Fixed

- APT repository packaging: added missing section field to .deb metadata
- Fixed reprepro config for automated apt repo deployment

---

## [0.3.0] - 2026-03-29

### Added

- **GPU acceleration (Vulkan)**: upgraded whisper-rs to 0.16 with Vulkan GPU support. Transcription is significantly faster on systems with compatible GPUs.
- **First-run onboarding wizard**: three-step guided setup on first launch. Microphone check, model download with progress, and hotkey confirmation.
- **Start on login**: new setting to launch Murmur automatically on desktop login via XDG autostart.
- **Auto-update system**: AppImage users receive in-app updates automatically via the Tauri updater plugin. Debian users can add the apt repository for updates via `apt upgrade`.
- **Multi-language support**: language selector with 19 languages plus auto-detect. Translation toggle to convert any language to English output. Multilingual whisper models added.
- **CI/CD pipeline**: automated lint, format, and build checks on every PR. Automated release pipeline builds signed .deb and .AppImage on version tags.

### Fixed

- Whisper hallucinations on VAD auto-stop (trailing silence trimmed before transcription)
- Default skin renamed from "Gemini V1" to "Comm Badge"
- All clippy warnings and rustfmt formatting issues resolved

---

## [0.2.0] - 2026-03-27

### Added

- **Wayland support**: text injection via wtype with automatic X11/Wayland detection at startup. Clipboard + Ctrl+V fallback for GNOME Wayland.
- **Voice activity detection (VAD)**: auto-stop recording after ~2 seconds of silence in tap mode. Configurable in settings.

### Fixed

- Pre-release security audit: all critical findings addressed

---

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
