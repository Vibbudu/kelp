# 📝 Kelp — Manual Testing Checklist

Use this comprehensive manual testing checklist to validate Kelp v0.1.0-alpha before public release.

---

## 🖥️ 1. Launcher Window & Global Hotkey
- [ ] **Summon Launcher**: Press `Alt + Space` from any context. The launcher window should appear instantly and center on the screen.
- [ ] **Dismiss Launcher (Escape)**: Press `Escape` while the window is focused. The launcher should hide immediately.
- [ ] **Dismiss Launcher (Lost Focus)**: Click anywhere outside the launcher window. The launcher should hide immediately.
- [ ] **Summon Toggle**: Press `Alt + Space` when the launcher is visible. It should hide immediately.
- [ ] **Rapid Summons**: Tap `Alt + Space` repeatedly and rapidly. Window should toggle visibility cleanly without flickering, scaling artifacts, or freezing.

---

## 🔍 2. Core Search and Filtering
- [ ] **Empty Query**: Verify that opening the launcher displays no results initially (clean UI).
- [ ] **Application Search**: Search for installed apps (e.g. `chrome`, `cmd`, `powershell`, `notepad`). Verify that they show up with high priority and display correct names and types.
- [ ] **Shortcut Search (`.lnk`)**: Search for shortcuts located on Desktop or Start Menu.
- [ ] **Folder Search**: Search for directories (e.g. `Documents`, `Downloads`). They should display correct `FOLDER` type labels.
- [ ] **Extension Filtering**:
  - [ ] Search `.pdf` -> displays all indexed PDF documents.
  - [ ] Search `.txt` -> displays all indexed plain text files.
  - [ ] Search `.docx` / `.xlsx` -> displays Office files.
  - [ ] Search `.rs` / `.toml` -> displays Rust source files.
- [ ] **Compound Search (Filter + Query)**: Search `.pdf resume` -> displays only PDF files containing the term "resume" (e.g., `Resume.pdf`).

---

## 🏆 3. Ranking & Learning Engine
- [ ] **Fuzzy Matching**: Type `vsc` -> should find `Visual Studio Code` (Acronym Match). Type `hlbr` -> should find `HeliumBrowser` (CamelCase Match).
- [ ] **Match Dominance**: Verify that exact name matches (e.g. `notepad`) outrank partial matches (e.g. `notepad++`) or fuzzy matches.
- [ ] **Selection Tracking (Learning)**:
  - [ ] Search for a query (e.g. `paint`).
  - [ ] Select a lower-ranked result (e.g., `Paint 3D`) and press `Enter`.
  - [ ] Reopen the launcher and type the same query (`paint`). Verify that `Paint 3D` is now ranked at the top.
- [ ] **No History Dominance Lock**: Verify that a newly indexed file with a perfect name match still appears in search results and is not hidden by older selections.

---

## ⚠️ 4. Edge Cases & Robustness
- [ ] **Unicode Filenames**: Index and search files containing emojis or non-ASCII characters (e.g. `R&D_Report_2026.pdf`, `résumé.txt`, `测试文档.docx`). Slicing logic must not crash.
- [ ] **Very Long Filenames**: Test searching files with paths/names exceeding 200+ characters. Ensure layout handles name truncation (`...`) cleanly without overflowing.
- [ ] **Rapid Typing**: Type a long search query extremely fast. Verify search results update fluidly without delay or UI freeze.
- [ ] **Rapid Backspacing**: Press and hold `Backspace` to clear a long query. Verify results adjust and clear cleanly back to empty state.
- [ ] **Offline Status**: Run the launcher offline (unconnected to internet). Verify Google Fonts/CDNs fall back gracefully.

---

## 📦 5. Installer & Uninstaller Validation
- [ ] **Fresh Installation**:
  - [ ] Run `KelpSetup-v0.1.0-alpha.exe` on a machine without Kelp.
  - [ ] Verify install path defaults to `C:\Program Files\Kelp`.
  - [ ] Verify Start Menu shortcut "Kelp" is created.
  - [ ] Verify optional Desktop shortcut is created.
- [ ] **Launch Verification**:
  - [ ] Launch from Start Menu shortcut.
  - [ ] Launch from Desktop shortcut.
  - [ ] Launch directly from `C:\Program Files\Kelp\kelp.exe`.
- [ ] **Upgrade Path**:
  - [ ] Run the installer again while Kelp is running. Verify that it detects the running application and prompts to close it automatically.
  - [ ] Verify upgrade completes successfully without losing existing `config.json` files.
- [ ] **Clean Uninstallation**:
  - [ ] Go to Windows Settings > Apps > Installed Apps.
  - [ ] Uninstall "Kelp".
  - [ ] Verify that all files are removed from `C:\Program Files\Kelp`.
  - [ ] Verify no locked file errors occur during uninstall.

---

## ⚡ 6. System Resource Usage
- [ ] **Idle CPU Usage**: Open Task Manager. Verify that Kelp background process consumes **0.0% CPU** while idle in the tray/background.
- [ ] **Memory Footprint**: Check private working set in Task Manager. Memory usage should remain **below 35 MB** during search operations.
- [ ] **Search Latency**: Check debug stats panel (if debug mode is enabled). Search timing should print **< 1.0 ms** for candidate lookup.
- [ ] **No Disk Thrashing**: Verify that searching does not trigger disk read/write activity (confirming database is queried in-memory only).
