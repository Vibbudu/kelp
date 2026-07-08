# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-alpha] - 2026-07-08

Initial public alpha release of **Nova Launcher**.

### Added
- **Core Search Engine**: Multi-tiered search matching including Exact, Prefix, Acronym, Camel Case, Contains, Fuzzy, and Typo matches.
- **Fast In-Memory Matching**: Zero-copy candidate evaluation under Read locks for sub-millisecond query search response.
- **Mica Styling UI**: Beautiful transparent CSS row layouts automatically syncing dark/light/system theme preferences.
- **Configurable Extension Whitelists**: Clean index storage by filtering and keeping only user-defined supported extensions.
- **Sleep-Proof Wakeup Thread**: Background OS event loop receiver for global `Alt + Space` hotkey trigger reliability.
- **Diagnostics Dashboard**: Visual memory usages, search speeds, index size, and query cache diagnostics (in Debug builds).
- **Subsystem Self-Validation**: Automatic startup validation checking databases, indexes, watch processes, and search capabilities.
- **Structured File Logger**: Routed logger tracking indexer, search, error, and startup modules with auto-rotation.
- **Global Panic Handler**: Catch-all hook formatting stack trace crash dumps to `logs/panic.log`.
