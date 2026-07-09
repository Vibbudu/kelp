# Contributing to Nova Launcher

Thank you for choosing to contribute to Nova Launcher! We welcome help in fixing bugs, improving search quality, optimizing performance, or polishing user interfaces.

---

## 🛠️ Code Structure

- `/src/lib.rs` - Library module definitions and re-exports.
- `/src/main.rs` - Main executable startup, window initialization, hotkey hook, and Event Loop.
- `/src/search.rs` - Precedence-based query parsing and string matching algorithms.
- `/src/ranking_engine.rs` - Scoring signals (base match, file priority, short path, system apps, learning stats).
- `/src/ui.html` - Self-contained HTML frontend rendering and JS keyboard row row-selection bridges.
- `/src/logger.rs` - File-based logging router, structured timestamps, and panic recovery hook.

---

## 🚀 How to Contribute

### 1. Fork the Repository
Create a personal fork of the repository and clone it locally:
```bash
git clone https://github.com/Vibbudu/nova.git
cd nova
```

### 2. Branching Strategy
We recommend naming branches logically:
- Features: `feature/your-feature-name`
- Bug fixes: `bugfix/issue-description`

### 3. Build & Test
Ensure your code changes pass compilation and all automated unit tests before opening a Pull Request:
```bash
cargo check
cargo test
```

### 4. Open a Pull Request
Submit your PR to the `develop` branch (or `main` branch depending on release strategies). Provide a clear description of the modifications and any bugs resolved.
