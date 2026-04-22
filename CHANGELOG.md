# Changelog

All notable changes to Murmur will be documented in this file.

## [0.3.5](https://github.com/murmurlinux/murmur/compare/murmur-v0.3.4...murmur-v0.3.5) (2026-04-22)


### Features

* add auto-update system (Tauri updater + apt repo) ([#17](https://github.com/murmurlinux/murmur/issues/17)) ([4ea422f](https://github.com/murmurlinux/murmur/commit/4ea422f1302f79ed34c72ad164bcd71a611166c9))
* add Ctrl+Shift+, shortcut to open settings ([#31](https://github.com/murmurlinux/murmur/issues/31)) ([beb1b6e](https://github.com/murmurlinux/murmur/commit/beb1b6e04e328a4293fdba0eab6fca9b2672039b))
* add first-run onboarding wizard ([#16](https://github.com/murmurlinux/murmur/issues/16)) ([64ab7fc](https://github.com/murmurlinux/murmur/commit/64ab7fca7c67651273d53e3f3819a66865bdaded))
* add multi-language support and translation ([#19](https://github.com/murmurlinux/murmur/issues/19)) ([036ce7f](https://github.com/murmurlinux/murmur/commit/036ce7f044ded5282ada32de96535fa53af9b8da))
* add start-on-login setting ([#14](https://github.com/murmurlinux/murmur/issues/14)) ([ccdbd82](https://github.com/murmurlinux/murmur/commit/ccdbd82a6033c1b1bc335e9302af0ab9202b8076))
* auto-stop recording on silence in tap mode (VAD) ([#4](https://github.com/murmurlinux/murmur/issues/4)) ([e4d5dcb](https://github.com/murmurlinux/murmur/commit/e4d5dcbb11ef6081dc0a679f7c5ad5e08d07c5cd))
* **ci:** add blind property test integration ([#28](https://github.com/murmurlinux/murmur/issues/28)) ([e718791](https://github.com/murmurlinux/murmur/commit/e718791debf7cbcf03f3b0d44a139629548d68a0))
* dynamic tray tooltip reflecting recording state ([#32](https://github.com/murmurlinux/murmur/issues/32)) ([8fc4300](https://github.com/murmurlinux/murmur/commit/8fc43003b0abcd6aacc5d94592807b25c57aef2f))
* **inject:** add Wayland support via wtype with auto-detection ([#3](https://github.com/murmurlinux/murmur/issues/3)) ([5c43f46](https://github.com/murmurlinux/murmur/commit/5c43f462b1fab6e1893738951f864d9a2ebfbe1f))
* MVP polish — popup, tap mode, show/hide, settings redesign ([#1](https://github.com/murmurlinux/murmur/issues/1)) ([ca4fdb1](https://github.com/murmurlinux/murmur/commit/ca4fdb1726235e5c1fd4f306ea1acb5e56d5c493))
* pluggable STT engine trait with LocalWhisperEngine ([#39](https://github.com/murmurlinux/murmur/issues/39)) ([4d19172](https://github.com/murmurlinux/murmur/commit/4d191729bead9fe5d4d9690bb48080c220e1b6f4))
* **popup:** tie pill accent to user's colour picker ([4e62ed2](https://github.com/murmurlinux/murmur/commit/4e62ed232e5a7b3fda64b6adb038dfb9a18ec1c0))
* redesign onboarding wizard ([#25](https://github.com/murmurlinux/murmur/issues/25)) ([ea3c4b0](https://github.com/murmurlinux/murmur/commit/ea3c4b0f1a01703c7f30603edd2eeb37c601166b))
* release-please + draft-first releases + cross-repo dispatch ([#54](https://github.com/murmurlinux/murmur/issues/54)) ([4383865](https://github.com/murmurlinux/murmur/commit/43838656f160d94b131cd55ddc4d207c394ccf72))
* Slice 1 — transparent skinned gadget window with interactive zones ([58e17bd](https://github.com/murmurlinux/murmur/commit/58e17bd13719bbf927ca5f0434e5f7ea6a91b752))
* Slice 2 — audio capture pipeline with live waveform ([c41c363](https://github.com/murmurlinux/murmur/commit/c41c3635f904e4a96094dfa0a1d832a9eb1bb4a2))
* Slice 3 — whisper transcription, text injection, global hotkey ([d73b653](https://github.com/murmurlinux/murmur/commit/d73b653fe0facc75a1d137a61493741aef445133))
* Slice 4 — settings, click-through, accent colour, packaging ([#1](https://github.com/murmurlinux/murmur/issues/1)) ([dde6c29](https://github.com/murmurlinux/murmur/commit/dde6c29ebf940ec4ebc3198bc322ff1f8d92cc40))
* upgrade whisper-rs from 0.12 to 0.16 ([#13](https://github.com/murmurlinux/murmur/issues/13)) ([6d7f5ec](https://github.com/murmurlinux/murmur/commit/6d7f5ecaf67d7264f8f62c779d23b41f05561409))


### Bug Fixes

* address pre-release audit findings ([#5](https://github.com/murmurlinux/murmur/issues/5)) ([1103050](https://github.com/murmurlinux/murmur/commit/110305022e55c7ffdb3528735e6107cbc16d1785))
* address pre-release audit findings (app repo) ([#21](https://github.com/murmurlinux/murmur/issues/21)) ([2364585](https://github.com/murmurlinux/murmur/commit/2364585da81e2329aef92570fa3ad5d2e014edf3))
* **ci:** add explicit permissions to CI workflow ([#37](https://github.com/murmurlinux/murmur/issues/37)) ([bb2b0fa](https://github.com/murmurlinux/murmur/commit/bb2b0fa280a02ab607b723b2f1993ebd21bdcd49))
* **ci:** fix reprepro distributions config indentation and add Suite field ([d3becb6](https://github.com/murmurlinux/murmur/commit/d3becb69c08a5ebe54f915319aa9a73732631a2c))
* **ci:** fix YAML syntax error in release workflow heredoc ([7d47035](https://github.com/murmurlinux/murmur/commit/7d47035e3baf3340e1d6e1a11ae97cf9d4c5d9fd))
* **ci:** release-please config cannot use '..' in paths ([#55](https://github.com/murmurlinux/murmur/issues/55)) ([1a34193](https://github.com/murmurlinux/murmur/commit/1a3419391facc11f6a7b4abb53ab7e2801e32368))
* **ci:** use APT_DEPLOY_TOKEN for cross-repo apt deploy ([#22](https://github.com/murmurlinux/murmur/issues/22)) ([5205323](https://github.com/murmurlinux/murmur/commit/5205323f22e36d00b312adec02d2ac5133834cd9))
* correct comparison table and remove misleading claims ([#35](https://github.com/murmurlinux/murmur/issues/35)) ([554a821](https://github.com/murmurlinux/murmur/commit/554a8218c78dce9635d65f08b34e9ec3d3b483be)), closes [#27](https://github.com/murmurlinux/murmur/issues/27)
* default accent colour to brand green ([#10](https://github.com/murmurlinux/murmur/issues/10)b981) ([08ea07e](https://github.com/murmurlinux/murmur/commit/08ea07e6f83e4fe2bfc68c5d7b0de2fe8df46979))
* **popup:** add backgroundColor transparent config, force size via API ([071d8be](https://github.com/murmurlinux/murmur/commit/071d8be5e95c700be3e35ff66f16c6389e4cd48d))
* **popup:** declare in tauri.conf.json for proper Linux transparency ([b29cbc6](https://github.com/murmurlinux/murmur/commit/b29cbc6ed9411e2c672baed36f17be8dcd611bd6))
* **popup:** raise pill position higher from screen bottom ([4b0d494](https://github.com/murmurlinux/murmur/commit/4b0d494a161b9ad668b51e3f9e28483149487750))
* **popup:** shrink window to 180x44 with minWidth/minHeight overrides ([c9e00da](https://github.com/murmurlinux/murmur/commit/c9e00daec6878f09c26a6ac7af261edd34b03a66))
* **popup:** transparent background so only pill shape is visible ([f3749c6](https://github.com/murmurlinux/murmur/commit/f3749c686472be846c1b63a01924a8c1e2ee7c17))
* **popup:** use large transparent window with pill at bottom-center ([e53e109](https://github.com/murmurlinux/murmur/commit/e53e109b021e809388398efdfc287a9bf25cca7d))
* rename default skin from Gemini V1 to Comm Badge ([#11](https://github.com/murmurlinux/murmur/issues/11)) ([1416802](https://github.com/murmurlinux/murmur/commit/141680252afbaea3d1cadb1cd3ab447ef7fa6c49))
* resolve all remaining audit findings (22/22 complete) ([5398a93](https://github.com/murmurlinux/murmur/commit/5398a93fc950c460d597d75640eefce3d7d8d3e9))
* support i16 and u16 audio devices in capture ([#43](https://github.com/murmurlinux/murmur/issues/43)) ([692d0ba](https://github.com/murmurlinux/murmur/commit/692d0bac9520092431e081254b38d80e9ed48020)), closes [#68](https://github.com/murmurlinux/murmur/issues/68)
* trim trailing silence to prevent Whisper hallucinations on VAD stop ([#12](https://github.com/murmurlinux/murmur/issues/12)) ([4dff6ae](https://github.com/murmurlinux/murmur/commit/4dff6ae4e84af1ef199f64b37cb8eff6da82e5dc))
* update binary size claims from ~3MB to ~5MB ([#2](https://github.com/murmurlinux/murmur/issues/2)) ([8f8d48a](https://github.com/murmurlinux/murmur/commit/8f8d48a379a1e97b7f0706e116fe79cc709330a6))
* UX polish quick wins (icons, tray, window sizes) ([#24](https://github.com/murmurlinux/murmur/issues/24)) ([5ece695](https://github.com/murmurlinux/murmur/commit/5ece695be9d282249659e4c792c50f0fe14fa4e2))
* v0.3.1 APT repo packaging and deployment ([a1a484e](https://github.com/murmurlinux/murmur/commit/a1a484ed76e7df779addc6c75d832b2803bd8366))

## [0.3.4] - 2026-04-20

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
