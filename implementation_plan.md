# Implementation Plan - Pre-Alpha Release 2 Stabilization

This plan outlines the changes to prepare Kelp for its second pre-alpha release. The goals are to add a background task tray icon for clean exit/summon management, fix extension filter searches (e.g. `.pdf`), prevent rotational wrap-around during keyboard results navigation, and refine CSS animations.

## User Review Required

> [!IMPORTANT]
> The system tray integration requires adding the `tray-icon` crate dependency to `Cargo.toml`. This is a lightweight, standard library maintained by the Tauri organization.

> [!NOTE]
> Pressing the window close button (or Alt+F4) will now hide Kelp to the system tray rather than terminating the process. The process can be exited cleanly via the System Tray context menu.

## Proposed Changes

---

### Component: Dependencies

#### [MODIFY] [Cargo.toml](file:///c:/Users/vibbu/Documents/engine/Cargo.toml)
* Add `tray-icon = "0.24"` to the `[dependencies]` section.

---

### Component: Keyboard Navigation and Animations

#### [MODIFY] [src/ui.html](file:///c:/Users/vibbu/Documents/engine/src/ui.html)
* **Rotation Clamp**: In the keydown handler for `ArrowDown` and `ArrowUp`, replace the modulo wrapping logic with clamping limits so that scrolling stops at the first and last items.
* **Instant Snapping**: Temporarily disable CSS transitions on the `selection-indicator` when `queryChanged` is true, ensuring the selection indicator snaps instantly to index 0 on new query inputs instead of sliding from legacy search positions.

---

### Component: Search Cache Engine

#### [MODIFY] [src/result_cache.rs](file:///c:/Users/vibbu/Documents/engine/src/result_cache.rs)
* **Extension Filter Cache Check**: In `get_longest_prefix_subset`, parse both the cached query and current query using `crate::query_parser::parse_query`. If their `extension_filter` values do not match (e.g. going from `.p` to `.pd` to `.pdf`), bypass the subset cache to run a full query search. This ensures correct candidates are indexed when typing extensions.

---

### Component: Main Subsystem (System Tray & Exit Hook)

#### [MODIFY] [src/main.rs](file:///c:/Users/vibbu/Documents/engine/src/main.rs)
* **Define Tray User Events**: Add `TrayIcon(tray_icon::TrayIconEvent)` and `Menu(tray_icon::menu::MenuEvent)` to the `UserEvent` enum.
* **Embed Icon Resource**: Add `load_icon_from_memory()` to load `assets/logo.png` from bytes using `image::load_from_memory` and convert it to `tray_icon::Icon` for runtime display.
* **Build System Tray**: Initialize the tray menu (items: "Show Kelp", "Exit Kelp") and register the tray icon alongside the event loop in `main()`.
* **Register Tray Event Handlers**: Call `TrayIconEvent::set_event_handler` and `MenuEvent::set_event_handler` to forward events asynchronously into the event loop via `EventLoopProxy`.
* **Handle Tray Events**:
  * Clicking tray icon (Left click): Toggle window visibility and focus.
  * Clicking "Show Kelp": Show and focus window.
  * Clicking "Exit Kelp": Set `*control_flow = ControlFlow::Exit` to terminate the background process.
* **WindowEvent Close Requested Hook**: Update `WindowEvent::CloseRequested` to hide the window instead of exiting the process.

## Verification Plan

### Automated Tests
Run standard cargo tests to verify query parses and matching limits:
```bash
cargo test
```

### Manual Verification
1. **System Tray**: Verify that running the app adds a Kelp icon to the notification area. Right-click it and click "Exit" to ensure it shuts down. Click "Show Kelp" to ensure it opens.
2. **Close Action**: Focus the launcher and press `Alt + F4` or click close. Verify that the process remains running in Task Manager/Tray and can be summoned again.
3. **Clamped Navigation**: Open the results list. Press Arrow Down repeatedly. Verify selection stops at the last row instead of wrapping to the first row. Press Arrow Up at the top row and verify it stops there.
4. **Extension Search**: Type `.pdf`. Verify all PDF documents in the Whitelisted folders are returned. Type `.lnk`. Verify all shortcuts are returned.
