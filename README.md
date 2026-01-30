# Sklad 📦

> Industrial-grade secure snippet warehouse for your system tray.

<!-- TODO: Add demo GIF here -->
<!-- ![Demo](public/demo.gif) -->

<p align="center">
  <i>🎬 Demo GIF coming soon...</i>
</p>

---

## ⬇️ Download

[![GitHub Release](https://img.shields.io/github/v/release/Rench321/sklad?style=for-the-badge)](https://github.com/Rench321/sklad/releases/latest)

| Windows | macOS (Apple Silicon) | macOS (Intel) | Linux |
|:-------:|:---------------------:|:-------------:|:-----:|
| [📦 .msi](https://github.com/Rench321/sklad/releases/latest) | [📦 .dmg (ARM)](https://github.com/Rench321/sklad/releases/latest) | [📦 .dmg (x64)](https://github.com/Rench321/sklad/releases/latest) | [📦 .deb](https://github.com/Rench321/sklad/releases/latest) |

> ⚠️ **macOS users:** If you see "Sklad is damaged", run: `xattr -cr /Applications/Sklad.app`

---

## Why Sklad?

| Feature | Sklad | Maccy | Text File |
|---------|:-----:|:-----:|:---------:|
| 🔐 Encrypted secrets | ✅ | ❌ | ❌ |
| 🦀 Memory safe (Rust) | ✅ | ✅ | N/A |
| 🖥 Cross-platform | ✅ | ❌ Mac only | ✅ |
| ☁️ No cloud/tracking | ✅ | ✅ | ✅ |
| 📁 Folder organization | ✅ | ❌ | ❌ |
| 🔍 Fast search | ✅ | ✅ | ❌ |
| 🖱 System tray access | ✅ | ✅ | ❌ |

---

## Features

- 🔒 **Master Password Protection** — Secrets are AES-256 encrypted
- 📋 **One-Click Copy** — Click tray, select snippet, done
- 🌙 **Dark/Light Theme** — Easy on the eyes
- 💾 **Local-Only Storage** — Your data never leaves your machine
- 📁 **Folder Organization** — Keep your snippets tidy
- ⌨️ **Quick Search** — Find anything instantly

---

## Build from Source

### Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/)

**Windows:** Also needs [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

**Linux:** Also needs:
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev
```

### Build

```bash
git clone https://github.com/Rench321/sklad.git
cd sklad
pnpm install
pnpm tauri build
```

Binaries will be in `src-tauri/target/release/bundle/`.

---

## License

[MIT](LICENSE) — Use it however you want.
