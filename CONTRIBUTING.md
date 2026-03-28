# Contributing to Murmur

Thanks for your interest in contributing to Murmur! This guide covers everything you need to get started.

## Development Setup

### Prerequisites

- **Rust** (stable, latest)
- **Node.js** 18+ with pnpm
- **System libraries:**

```bash
# Ubuntu / Debian
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev xdotool

# Fedora
sudo dnf install webkit2gtk4.1-devel libayatana-appindicator-devel xdotool

# Arch
sudo pacman -S webkit2gtk-4.1 libayatana-appindicator xdotool
```

### Build & Run

```bash
git clone https://github.com/murmurlinux/murmur.git
cd murmur
pnpm install
pnpm tauri dev     # development mode with hot reload
pnpm tauri build   # production build
```

## Project Structure

```
murmur/
├── src/                    # SolidJS frontend
│   ├── components/         # UI components (SkinRenderer, Waveform, etc.)
│   ├── lib/                # Utilities (settings, skin loader, colour)
│   └── assets/skins/       # Skin images + config
├── src-tauri/              # Rust backend
│   └── src/
│       ├── audio/          # Audio capture (cpal)
│       ├── commands/       # Tauri IPC commands
│       ├── inject/         # Text injection (xdotool)
│       ├── stt/            # Speech-to-text (whisper.cpp)
│       └── lib.rs          # App setup + state
├── design/                 # Design mockups + assets (not tracked)
└── .github/                # Community templates
```

## Making Changes

1. **Fork** the repo and create a branch: `git checkout -b feature/your-feature`
2. **Make your changes**:follow existing code patterns
3. **Test**:make sure the app builds and runs: `pnpm tauri dev`
4. **Commit**:use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `chore:`
5. **Push** and open a **Pull Request**

### Commit Format

```
type: short description

Optional longer description explaining the why.
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Code Style

- **Rust**:follow `rustfmt` defaults. Run `cargo fmt` before committing.
- **TypeScript**:follow existing patterns. No semicolons, single quotes.
- **CSS**:Tailwind utility classes in components, custom CSS in `styles.css`.

## Creating Skins

Skins are defined by two files in `src/assets/skins/<skin-name>/`:

- `body.png`:the skin image (transparent background, ~1380x752)
- `skin.json`:interactive zones, LED positions, accent colour config

See the existing `comm-badge` skin for reference. Skin development docs coming soon.

## Reporting Issues

Use [GitHub Issues](https://github.com/murmurlinux/murmur/issues) with the appropriate template:

- **Bug report**:include your distro, desktop environment, and steps to reproduce
- **Feature request**:describe the use case and proposed solution

## Code of Conduct

Be respectful, constructive, and inclusive. We're building something for everyone.

## License

By contributing, you agree that your contributions will be licensed under [GPL-3.0](LICENSE).
