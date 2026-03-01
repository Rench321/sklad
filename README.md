# Sklad 📦

[![GitHub Release](https://img.shields.io/github/v/release/Rench321/sklad?style=flat-square)](https://github.com/Rench321/sklad/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey?style=flat-square)]()
[![Made with Rust](https://img.shields.io/badge/Made%20with-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)

**Sklad** is a cross-platform snippet manager that lives in your system tray. Store passwords, API keys, code snippets, and any text you copy frequently — encrypted and always one click away.

<p align="center">
  <img src="public/demo.gif" alt="Sklad Demo" width="600">
</p>

---

## ✨ Features

- 🔒 **Master Password Protection** — Secrets are AES-256 encrypted with Argon2 key derivation
- 📋 **One-Click Copy** — Click tray → select snippet → done
- 🔍 **Global Search** — Find anything instantly from anywhere with custom hotkey
- ⚡ **Instant Creation** — Spawn the snippet creation window from any app with custom hotkey
- 📁 **Folder Organization** — Organize snippets into nested folders
- ⏱️ **Auto-Lock & Autosave** — Configurable vault timeout and unsaved changes handling
- 🌙 **Dark/Light Theme** — Easy on the eyes
- 💾 **Local-Only Storage** — Your data never leaves your machine
- 🖱 **Customizable Tray** — Configure tray left-click actions and context menu layout

<p align="center">
  <img src="public/screenshot.png" alt="Sklad UI" width="700">
</p>

---

## ⬇️ Download

[![GitHub Release](https://img.shields.io/github/v/release/Rench321/sklad?style=for-the-badge)](https://github.com/Rench321/sklad/releases)

| Windows | macOS (Apple Silicon) | macOS (Intel) | Linux |
|:-------:|:---------------------:|:-------------:|:-----:|
| [📦 .msi](https://github.com/Rench321/sklad/releases) | [📦 .dmg (ARM)](https://github.com/Rench321/sklad/releases) | [📦 .dmg (x64)](https://github.com/Rench321/sklad/releases) | [📦 .deb](https://github.com/Rench321/sklad/releases) |

### 🍺 Homebrew (macOS)
You can also install Sklad via Homebrew:
```bash
brew install --cask --no-quarantine Rench321/sklad/sklad
```

Or by tapping the repository first:
```bash
brew tap Rench321/sklad
brew install --cask --no-quarantine sklad
```

To update Sklad via Homebrew:
```bash
brew update
brew upgrade --cask sklad
```

> [!WARNING]
> **Windows users:** You may see a SmartScreen warning because the app is not code-signed yet and has few downloads. Click *"More info"* → *"Run anyway"* to proceed.

> [!WARNING]
> **macOS users:** The app is not notarized yet (requires Apple Developer account).  
> If you see *"Sklad is damaged"*, open Terminal and run:
> ```bash
> xattr -cr /Applications/Sklad.app
> ```

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

## 🌟 Featured In

- [Hacker News](https://news.ycombinator.com/item?id=46854028)
- [Korben.info (French)](https://korben.info/sklad-gestionnaire-snippets-chiffres-tray.html)

---

## 🛠 Build from Source

### Prerequisites

- [Rust](https://rustup.rs/)
- [Node.js](https://nodejs.org/) (v18+)
- [pnpm](https://pnpm.io/)

<details>
<summary><b>Windows</b></summary>

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "Desktop development with C++" workload.
</details>

<details>
<summary><b>Linux (Debian/Ubuntu)</b></summary>

```bash
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```
</details>

<details>
<summary><b>macOS</b></summary>

```bash
xcode-select --install
```
</details>

### Build

```bash
git clone https://github.com/Rench321/sklad.git
cd sklad
pnpm install
pnpm tauri build
```

Binaries will be in `src-tauri/target/release/bundle/`.

---

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit PRs.

---


## 👥 Contributors

<a href="https://github.com/olejsc">
  <img src="https://github.com/olejsc.png" width="50" style="border-radius: 50%;" alt="olejsc"/>
</a>
<a href="https://github.com/overflowy">
  <img src="https://github.com/overflowy.png" width="50" style="border-radius: 50%;" alt="overflowy"/>
</a>
<a href="https://github.com/beparmentier">
  <img src="https://github.com/beparmentier.png" width="50" style="border-radius: 50%;" alt="beparmentier"/>
</a>
<a href="https://github.com/marsender">
  <img src="https://github.com/marsender.png" width="50" style="border-radius: 50%;" alt="marsender"/>
</a>
<a href="https://github.com/IMNotMax">
  <img src="https://github.com/IMNotMax.png" width="50" style="border-radius: 50%;" alt="IMNotMax"/>
</a>

---

## 📄 License

[MIT](LICENSE) — Use it however you want.

---

<p align="center">
  Made with 🦀 Rust + ⚛️ React + 💙 Tauri
</p>
