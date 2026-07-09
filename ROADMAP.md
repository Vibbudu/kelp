# Roadmap

This document outlines the planned milestones, upcoming features, and future goals for **Kelp**.

---

## 🗺️ Future Milestones

### Phase 1: Core Consolidation & Release (Current)
- [x] High-performance zero-copy memory search
- [x] Standard file type indexing and whitelisting
- [x] Alt+Space hotkey reliability
- [x] Structured diagnostics logs and panic handles

### Phase 2: User Customization & Controls (Next)
- **Settings Panel**: Interactive configuration UI to select whitelisted extensions, watch folders, and color schemes.
- **Excluded Directories Filter**: Blacklist custom folders (e.g. `C:\Temp\`, specific build folders) directly in the UI.
- **System Tray Agent**: Minimizing to system tray upon startup with customizable tray icons and context menus.

### Phase 3: Extension Providers & Custom Actions
- **Web Engines Queries**: Direct keyword queries (e.g., `g text` for Google search, `yt query` for YouTube search).
- **Calculator & Unit Conversion**: Instant math results computed directly inside the search bar.
- **System Commands**: Direct commands like `shutdown`, `restart`, `lock`, or `sleep`.

### Phase 4: Installers & Automated Deployments
- **Self-Updater**: Automatic background checking for updates and seamless updates downloading.
- **WiX Installer Package**: Generate `.msi` installers for standard Windows setup flows.
