# 📦 Kelp v0.1.0-alpha Release Assets

This document contains ready-to-use content for the GitHub Release page of Kelp's first public alpha release.

---

## 📄 Release Notes

```markdown
# 🚀 Kelp v0.1.0-alpha (Public Alpha)

We are excited to announce the first public alpha release of **Kelp**, a blazingly fast, lightweight, and modern keyboard-driven search launcher for Windows.

Powered by a custom Rust matching engine and a transparent HTML/JS webview frontend, Kelp provides instant access to your applications, shortcuts, folders, and documents with sub-millisecond query latency and zero idle CPU usage.

### ✨ Key Features
- ⚡ **Sub-millisecond Search**: Near-instant query parsing and memory-index matching.
- 🎨 **Mica-styled Glass UI**: Beautiful transparent interface with subtle blur and layout scaling.
- 🧠 **Adaptive Learning**: Results are dynamically re-ranked based on your selection frequency and recency.
- 🔍 **Rich Query Syntax**: Support for extension filters (e.g. `.pdf resume`) and acronym/camel-case shortcuts (e.g. `vsc` matches `Visual Studio Code`).
- 📁 **Comprehensive File Support**: Search files, folders, shortcuts, scripts, and documents seamlessly.
- 🔒 **Zero Background Idle CPU**: The file watcher works efficiently with low system overhead.

### 🛠️ What's New in this Release candidate (RC1)
- Overhauled matching algorithms to be 100% Unicode case-insensitive safe.
- Lowered score filtering thresholds to prevent missing relevant document matches.
- Balanced history signal contribution to prevent search favorites from overriding new exact matches.
- Added support for Windows scripts (`.bat`, `.cmd`, `.ps1`) and installers (`.msi`, `.msix`).
- Solved progressive typing cache narrowing issues.
- Integrated dynamic AppData storage support and fully silent background launching (no console windows in release mode).
```

---

## ⚠️ Alpha Warning

```markdown
> [!WARNING]
> **Pre-release Software**: This is a **Public Alpha** version of Kelp. While it has undergone extensive testing, it is still early-stage software. You may encounter unexpected behavior or bugs.
> If you experience issues or crashes, please report them using our GitHub Issue templates!
```

---

## 🛠️ Installation Instructions

```markdown
### 📥 How to Install

1. Download the installer: **`KelpSetup-v0.1.0-alpha.exe`** from the Assets section below.
2. Run the installer executable.
3. Follow the installation wizard. By default, Kelp will install under `Program Files\Kelp`.
4. Check the box to launch Kelp at the end of the installation.
5. Press **`Alt + Space`** to summon the launcher and begin searching.

### ⚙️ How to Configure
Kelp will generate a default configuration file named `config.json` in its Local AppData folder (`%LOCALAPPDATA%\Kelp\config.json`) on first startup. You can customize the `supported_extensions` array in this file to control which file formats are indexed.
```

---

## 🖥️ System Requirements

```markdown
### 📋 System Requirements
- **Operating System**: Windows 10 (Build 19041 or higher) or Windows 11.
- **Runtime**: WebView2 Runtime (pre-installed on Windows 11 and updated Windows 10 devices).
- **RAM**: ~30 MB free space.
- **Disk**: ~10 MB for installation, plus database cache footprint (depends on number of scanned files).
```

---

## 📝 Known Issues

```markdown
### 🔍 Known Issues & Limitations
- **Startup Sync Locking**: During initial/sync scans, new file watcher events might experience a minor memory-indexing delay. This does not affect search functionality of already indexed files.
- **Manual Path Configuration**: Custom search path configuration is currently edited via the SQLite DB or config overrides. UI configuration settings will be added in `v0.2.0-beta`.
```
