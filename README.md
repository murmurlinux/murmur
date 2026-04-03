<p align="center">
  <a href="https://murmurlinux.com">
    <img src=".github/banner.png" alt="Murmur: AI voice to text for Linux" width="100%">
  </a>
</p>

<p align="center">
  <a href="https://github.com/murmurlinux/murmur/releases"><img src="https://img.shields.io/github/v/release/murmurlinux/murmur?style=flat-square&color=14b8a6&label=version" alt="Version"></a>
  <a href="https://github.com/murmurlinux/murmur/blob/main/LICENSE"><img src="https://img.shields.io/github/license/murmurlinux/murmur?style=flat-square&color=14b8a6" alt="License"></a>
  <a href="https://github.com/murmurlinux/murmur/stargazers"><img src="https://img.shields.io/github/stars/murmurlinux/murmur?style=flat-square&color=f59e0b" alt="Stars"></a>
  <a href="https://github.com/murmurlinux/murmur/issues"><img src="https://img.shields.io/github/issues/murmurlinux/murmur?style=flat-square&color=14b8a6" alt="Issues"></a>
</p>

---

**Murmur** is a Linux-native AI voice-to-text desktop gadget. Hold a hotkey, speak, and text appears at your cursor in any application. Powered by [whisper.cpp](https://github.com/ggerganov/whisper.cpp) for fast, accurate, 100% offline transcription.

No cloud. No account. No telemetry. Your voice never leaves your machine.

## Features

- **100% Offline.** whisper.cpp runs locally on your CPU. Zero network requests after model download.
- **Floating Comm Badge.** A desktop gadget with customisable skins and accent colours. Always visible, always ready.
- **Universal Text Injection.** Types into any app via xdotool (X11) or wtype (Wayland). Terminals, IDEs, browsers, chat. If it has a cursor, Murmur types into it.
- **Hold or Tap to Record.** Configurable global hotkey. Hold to record and release to transcribe, or tap to toggle. Voice activity detection auto-stops when you finish speaking.
- **Multiple Models.** Tiny (75 MB, ~3s), Base (142 MB, ~8s), Small (466 MB, best accuracy). Choose your tradeoff.
- **Tiny Footprint.** ~15 MB .deb, ~50 MB RAM. Built with Rust + Tauri 2. Starts in under a second.

## Quick Install

**APT Repository (recommended, auto-updates):**

```bash
curl -fsSL https://murmurlinux.github.io/apt/gpg.key | sudo tee /etc/apt/keyrings/murmur.asc > /dev/null
echo "deb [signed-by=/etc/apt/keyrings/murmur.asc] https://murmurlinux.github.io/apt/ stable main" | sudo tee /etc/apt/sources.list.d/murmur.list
sudo apt update && sudo apt install murmur
```

Updates automatically via `sudo apt upgrade`.

**AppImage (portable, auto-updates on launch):**

```bash
wget https://github.com/murmurlinux/murmur/releases/download/v0.3.3/Murmur_0.3.3_amd64.AppImage
chmod +x Murmur_0.3.3_amd64.AppImage
./Murmur_0.3.3_amd64.AppImage
```

**.deb direct download (manual updates):**

```bash
wget https://github.com/murmurlinux/murmur/releases/download/v0.3.3/Murmur_0.3.3_amd64.deb
sudo dpkg -i Murmur_0.3.3_amd64.deb
```

**Uninstall:**

```bash
sudo apt remove murmur
```

> Requires: Linux (Ubuntu 22.04+, Fedora 38+, Arch), PipeWire or PulseAudio, xdotool (X11) or wtype (Wayland)

## Build from Source

```bash
# Prerequisites
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev xdotool wtype

# Clone and build
git clone https://github.com/murmurlinux/murmur.git
cd murmur
pnpm install
pnpm tauri build
```

The built binary will be in `src-tauri/target/release/murmur`.

## Usage

1. Launch Murmur. The Comm Badge widget appears on your desktop.
2. Press your hotkey (default: `Ctrl+Shift+Space`) and hold.
3. Speak naturally.
4. Release. Text appears at your cursor.

### Configuration

Open settings via the gear icon on the Comm Badge:

- **Hotkey.** Change the global shortcut.
- **Model.** Select Tiny, Base, or Small (auto-downloads on first use).
- **Accent colour.** Customise the Comm Badge glow.
- **Recording mode.** Hold-to-record or tap-to-toggle with silence auto-stop.
- **Show/hide skin.** Minimise to tray. A recording indicator popup appears when dictating with the skin hidden.

Settings are stored in `~/.local/share/com.murmurlinux.murmur/settings.json`.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Backend | Rust + Tauri 2 |
| Frontend | SolidJS + TypeScript |
| STT Engine | whisper.cpp (via whisper-rs) |
| Audio | cpal (PipeWire / PulseAudio) |
| Text Injection | xdotool (X11), wtype (Wayland) |
| Build | Vite 6 + Cargo |

## Whisper Models

| Model | Size | Speed | Accuracy |
|-------|------|-------|----------|
| tiny.en | 75 MB | ~3-4s | Good |
| base.en | 142 MB | ~8-10s | Better |
| small.en | 466 MB | ~20-30s | Best |

Models auto-download from Hugging Face on first use. SHA256 verified.

## Roadmap

- [x] Core dictation + settings (v0.1.0)
- [x] Tap-to-toggle recording mode (v0.1.1)
- [x] Show/hide skin + recording indicator popup (v0.1.1)
- [x] Settings redesign (v0.1.1)
- [x] Wayland support via wtype (v0.2.0)
- [x] Voice activity detection / silence auto-stop (v0.2.0)
- [x] GPU acceleration via Vulkan (v0.3.0)
- [x] First-run onboarding wizard (v0.3.0)
- [x] Start on login (v0.3.0)
- [x] Auto-update system (v0.3.0)
- [x] Multi-language support + translation (v0.3.0)
- [x] Settings keyboard shortcut (v0.3.3)
- [x] Dynamic tray tooltip (v0.3.3)
- [ ] Transcript history (Pro)
- [ ] Voice commands (Pro)
- [ ] Cloud STT: Groq Whisper + Deepgram Nova-3 (Pro)
- [ ] LLM text cleanup (Pro)
- [ ] Custom dictionaries / hot words (Pro)
- [ ] CLI mode: murmur-cli (Pro)
- [ ] Premium skins (Pro)

See the full [roadmap](https://murmurlinux.com/about) on our website.

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, build instructions, and PR guidelines.

## License

[GPL-3.0](LICENSE). Free and open source. Read the code, verify the privacy claims, contribute features.

---

<p align="center">
  <a href="https://murmurlinux.com">Website</a> &nbsp;&middot;&nbsp;
  <a href="https://github.com/murmurlinux/murmur/issues">Issues</a> &nbsp;&middot;&nbsp;
  <a href="https://github.com/murmurlinux/murmur/discussions">Discussions</a>
</p>
