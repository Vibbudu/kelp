<div align="center">
  <img src="assets/logo.png" alt="Kelp Logo" width="96" />

  <h1>Kelp</h1>

  <p>
    A fast, lightweight launcher for Windows.
    <br />
    Find anything. Open anything. Get out of the way.
  </p>

  <p>
    <a href="https://kelp-launcher.vercel.app/">Website</a>
    &nbsp; · &nbsp;
    <a href="https://github.com/Vibbudu/kelp/releases">Download</a>
    &nbsp; · &nbsp;
    <a href="https://github.com/Vibbudu/kelp/issues">Issues</a>
  </p>

  <p>
    <img src="https://img.shields.io/badge/status-alpha-orange" alt="Alpha" />
    <img src="https://img.shields.io/badge/platform-Windows-brightgreen" alt="Windows" />
    <img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT" />
  </p>
</div>

---

## About

Kelp is a keyboard-driven launcher for Windows.

Press `Alt + Space`, type what you're looking for, and open it.

It is built to be fast, simple, and lightweight, with real-time indexing and intelligent search that gets out of the way when you're done.

## Downloads

Kelp is currently in public alpha. Unexpected issues may occur.

<a href="https://github.com/Vibbudu/kelp/releases">Download Kelp</a>

## Features

* Fast search
* Fuzzy matching
* Application and file search
* Real-time indexing
* Search by file extension
* Usage-based result ranking
* Keyboard-first controls
* Low resource usage

## Keyboard shortcuts

| Shortcut      | Action           |
| ------------- | ---------------- |
| `Alt + Space` | Open Kelp        |
| `↑` `↓`       | Navigate results |
| `Enter`       | Open             |
| `Esc`         | Close            |

## Development

Kelp is built for Windows using Rust.

### Requirements

* Windows 10 or later
* Rust and Cargo

### Build

```bash
git clone https://github.com/Vibbudu/kelp.git
cd kelp
cargo run
```

For a release build:

```bash
cargo build --release
```

## Roadmap

Kelp is actively developed.

Planned features include:

* System tray support
* Web search shortcuts
* Custom indexing paths
* Calculator and unit conversion
* More ways to customize search

## Contributing

Contributions, ideas, and bug reports are welcome.

Open an issue or submit a pull request to get involved.

## License

Kelp is released under the MIT License.
