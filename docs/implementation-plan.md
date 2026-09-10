# Kelp Engineering Implementation Plan: Search Performance, Exclusions & First-Run Onboarding

**Document Version:** 1.0.0  
**Target Platform:** Windows 10 & 11 (x86_64, WebView2, Tao, Wry, SQLite, Tokio)  
**Status:** Approved for Implementation  

---

## 1. Executive Summary & Architectural Decisions

Following our thorough code audit and bottleneck analysis across the search and indexing pipelines, this document outlines the engineering blueprint to transition Kelp from an unpruned linear scanner into a resilient, scalable, sub-millisecond desktop search engine capable of managing 100k+ files smoothly under 50 MB of RAM.

### Architectural Decisions (ADRs)

1. **ADR-01: Pruned Directory Traversal via `WalkDir::filter_entry`**
   - *Decision:* Replace unpruned `for entry in walker` with `.filter_entry(|e| !should_exclude_entry(e, &config))`.
   - *Rationale:* Calling `continue` inside a standard `for` loop in `walkdir` only skips the current entry but descends recursively into subtrees (`node_modules`, `.git`, `target`, etc.). Pruning at the directory boundary eliminates hundreds of thousands of needless filesystem checks.
   - *Attribute Optimization:* Extract file attributes directly from `DirEntry::file_type()` and `DirEntry::metadata()` instead of issuing independent `std::fs::symlink_metadata()` syscalls per file.

2. **ADR-02: Non-Blocking Startup & Asynchronous First-Run Indexing**
   - *Decision:* Never await full filesystem indexing in `UIBridge::initialize()` before creating the Tao window and WebView2.
   - *Rationale:* Awaiting initial indexing causes a 15–45s freeze on first launch. Moving initial and incremental sync to background Tokio tasks allows the UI to render in < 500ms with a non-intrusive indexing indicator.

3. **ADR-03: Sliding Debounce & Batching for Filesystem Watcher**
   - *Decision:* Introduce a 500ms sliding debounce channel in `FileWatcher` to coalesce file burst events (e.g., git checkouts, npm installs, build outputs).
   - *Rationale:* Current code spawns a separate Tokio task, executes an unbatched SQLite write, and triggers a full $O(N \log N)$ index re-sort for *every single file event*. Batching coalesces hundreds of events into a single SQLite transaction and an in-place index update.

4. **ADR-04: Monotonic Query Sequencing & Elimination of Redundant Rescan**
   - *Decision:* Introduce a monotonic `query_id: u64` in search IPC messages. Remove the entire duplicate 30-line `index.get_all()` rescan loop in `src/main.rs:489-512`.
   - *Rationale:* Out-of-order execution between fast and slow query keystrokes causes UI result flickering. Removing the redundant second full-index match loop eliminates 300ms–800ms of latency per keystroke at 100k scale.

5. **ADR-05: Stack-Allocated / Flat-Scratch Buffer for Fuzzy Matching**
   - *Decision:* Replace dynamic 2D vector allocation (`vec![vec![f64::MIN; t_len]; q_len]`) in `compute_fuzzy_match()` with a reusable flat scratch buffer.
   - *Rationale:* Prevents up to 200,000 heap vector allocations on query misses. Pre-filter candidates with character bitmasks before dynamic programming.

6. **ADR-06: Configurable Exclusions with In-Place Memory Purge**
   - *Decision:* Wire `config.excluded_paths` into both the crawler and runtime search. When an exclusion is added in Settings, immediately prune matching entries in `MemoryIndex` and execute a background SQLite `DELETE` without triggering a full, destructive reindex.

7. **ADR-07: First-Run Onboarding Flow**
   - *Decision:* Provide a clean, minimal first-launch onboarding modal in the WebView for critical initial choices (Hotkey: Alt+Space vs. Ctrl+Space, search directories, exclusions, startup behavior), accompanied by an unobtrusive indexing progress status pill in the launcher footer.

---

## 2. Implementation Phases & Affected Files

```
┌─────────────────────────────────────────────────────────────┐
│ Phase 1: Critical Bug Fixes & Concurrency Integrity         │
│ - Storage learning deletion fix (BUG-01)                    │
│ - Monotonic query_id & race prevention (BUG-05)             │
│ - Elimination of redundant get_all() rescan (BUG-04)        │
│ - Configuration synchronization fix (BUG-07)                │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               v
┌─────────────────────────────────────────────────────────────┐
│ Phase 2: Traversal Pruning & Database Batching              │
│ - WalkDir entry pruning with filter_entry (BUG-03)          │
│ - Single-pass metadata inspection (I/O optimization)        │
│ - Batched stale file deletions (BUG-09)                     │
│ - Fast UWP discovery caching (BUG-11)                       │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               v
┌─────────────────────────────────────────────────────────────┐
│ Phase 3: Watcher Coalescing & Index Optimizations           │
│ - 500ms sliding window event debouncer (BUG-06)             │
│ - In-place O(1) MemoryIndex mutations (BUG-14)              │
│ - Flat scratch buffer for Fuzzy DP (BUG-12)                 │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               v
┌─────────────────────────────────────────────────────────────┐
│ Phase 4: Configurable Exclusions & Instant Purge            │
│ - Engine-wide config.excluded_paths integration (BUG-08)    │
│ - Settings UI: Excluded Folders manager                     │
│ - Instant memory purge & batched DB delete                  │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               v
┌─────────────────────────────────────────────────────────────┐
│ Phase 5: Polished First-Run Onboarding & Progress UX        │
│ - Asynchronous initial indexing (BUG-02)                    │
│ - First-run onboarding wizard (Hotkeys, Folders, Autostart) │
│ - Non-intrusive status pill in launcher footer              │
└─────────────────────────────────────────────────────────────┘
```

---

### Phase 1: Critical Bug Fixes & Concurrency Integrity

#### Affected Files:
- `src/storage.rs`
- `src/main.rs`
- `src/ui_bridge.rs`
- `src/ui.html`

#### Changes Required:
1. **Fix SQLite Learning Purge ([`src/storage.rs`](src/storage.rs)):**
   - Update `clear_learning_data()` from `DELETE FROM query_selections` to:
     ```rust
     pub fn clear_learning_data(&self) -> Result<()> {
         let conn = self.get_conn();
         conn.execute("DELETE FROM search_history", [])?;
         conn.execute("DELETE FROM query_frequencies", [])?;
         Ok(())
     }
     ```
2. **Monotonic Query Sequencing ([`src/main.rs`](src/main.rs), [`src/ui.html`](src/ui.html)):**
   - Add `query_id: u64` to `IpcMessage::Search` and `UserEvent::SearchRequest`.
   - Maintain `latest_query_id: Arc<AtomicU64>` in the event loop.
   - Ignore/drop any search task result whose `query_id` is less than `latest_query_id.load()`.
3. **Eliminate Redundant Rescan Loop ([`src/main.rs`](src/main.rs)):**
   - Remove lines 489–512 in `src/main.rs` (the `get_all()` clone and second `match_file` iteration).
   - Unify search handling by routing directly through `UIBridge::search()`, which includes application/shortcut deduplication.
4. **Synchronize Config Updates ([`src/main.rs`](src/main.rs)):**
   - In `UserEvent::SaveSettingsRequest`, call `engine.update_config(new_cfg)` so `UIBridge` immediately adopts newly saved configurations.

---

### Phase 2: Traversal Pruning & Database Batching

#### Affected Files:
- `src/indexer.rs`
- `src/utilities.rs`

#### Changes Required:
1. **Prune WalkDir Traversal ([`src/indexer.rs`](src/indexer.rs)):**
   - Refactor `index_paths` to use `.filter_entry()`:
     ```rust
     let walker = WalkDir::new(path)
         .follow_links(false)
         .into_iter()
         .filter_entry(|e| !crate::utilities::should_exclude_dir_entry(e, &config));
     ```
   - If a directory name matches an exclusion (`node_modules`, `.git`, `target`, `AppData\Local\Temp`, etc.), WalkDir will not enter it.
2. **Single-Pass Metadata Extraction ([`src/utilities.rs`](src/utilities.rs)):**
   - Read attributes and file type directly from `walkdir::DirEntry` without invoking `std::fs::symlink_metadata()` on every path.
3. **Batched Deletion of Stale Files ([`src/indexer.rs`](src/indexer.rs), [`src/storage.rs`](src/storage.rs)):**
   - Add `Storage::delete_files_batch(&self, paths: &[String])` using a single transaction and prepared statement.
   - Replace one-by-one `delete_file()` calls in `index_paths` cleanup with a batched transaction.
4. **UWP Discovery Optimization ([`src/indexer.rs`](src/indexer.rs)):**
   - Avoid executing `powershell.exe Get-StartApps` on every minor sync. Cache UWP results and only refresh if explicitly requested via full reindex or if UWP cache is missing.

---

### Phase 3: Watcher Coalescing & Index Optimizations

#### Affected Files:
- `src/watcher.rs`
- `src/ui_bridge.rs`
- `src/memory_index.rs`
- `src/search.rs`

#### Changes Required:
1. **Sliding Debounce Event Coalescer ([`src/watcher.rs`](src/watcher.rs)):**
   - Implement an event debouncer thread that buffers events across a 500ms sliding window before emitting a `BatchUpdated { created_or_modified: Vec<PathBuf>, deleted: Vec<PathBuf> }`.
2. **Batched Watcher Application ([`src/ui_bridge.rs`](src/ui_bridge.rs)):**
   - Handle `BatchUpdated` in a single `tokio::task::spawn_blocking` task:
     - Batch write changes to SQLite in one transaction.
     - Apply changes to `MemoryIndex` in one pass.
     - Invalidate query cache once per batch.
3. **Optimize MemoryIndex & String Handling ([`src/memory_index.rs`](src/memory_index.rs)):**
   - Store a pre-lowercased name inside `FileMetadata` or an auxiliary index entry to eliminate string allocations during comparisons.
   - Remove redundant `rebuild_index()` full sorting on single item additions.
4. **Flat Scratch Buffer for Fuzzy Alignment ([`src/search.rs`](src/search.rs)):**
   - Replace `vec![vec![f64::MIN; t_len]; q_len]` with a pre-allocated flat buffer or fixed stack array (`[f64; 256 * 64]`) for query lengths $\le 64$ and target names $\le 256$.
   - Pre-check character subsets using a u64 character bitmask before computing full DP scores.

---

### Phase 4: Configurable Exclusions & Instant Purge

#### Affected Files:
- `src/config.rs`
- `src/indexer.rs`
- `src/utilities.rs`
- `src/ui_bridge.rs`
- `src/ui.html`

#### Changes Required:
1. **Exclusions Configuration Integration ([`src/utilities.rs`](src/utilities.rs)):**
   - Update `should_exclude_path(path: &Path, config: &AppConfig) -> bool` to check user-defined `config.excluded_paths` in addition to sensible built-in system defaults (`node_modules`, `.git`, `target`, `AppData\Local\Temp`, etc.).
2. **Settings UI for Excluded Paths ([`src/ui.html`](src/ui.html)):**
   - Add an **"Excluded Folders & Paths"** card under the Indexing settings tab.
   - Support "Add Excluded Folder..." via native folder picker (`pick_folder` IPC).
   - Display active exclusions with individual remove buttons.
3. **Instant Purge Without Rebuild ([`src/ui_bridge.rs`](src/ui_bridge.rs), [`src/storage.rs`](src/storage.rs)):**
   - Add `UIBridge::add_exclusion(&mut self, path: &str)`:
     - MemoryIndex: `self.index.remove_prefix(path)` immediately in RAM.
     - Storage: `self.storage.delete_folder_recursive(path)` in background.
     - Query Cache: Clear results cache.
     - Result: Content disappears from search in < 50ms without re-scanning unchanged directories.

---

### Phase 5: Polished First-Run Onboarding & Progress UX

#### Affected Files:
- `src/main.rs`
- `src/ui_bridge.rs`
- `src/ui.html`

#### Changes Required:
1. **Asynchronous Initial Indexing ([`src/ui_bridge.rs`](src/ui_bridge.rs), [`src/main.rs`](src/main.rs)):**
   - Make `UIBridge::initialize` return immediately after opening SQLite and loading cached files (or starting with an empty index).
   - Launch the initial indexer on a detached background task.
   - Stream indexing progress via `UserEvent::IndexingProgress { indexed_count: usize, total_estimate: usize }`.
2. **First-Run Onboarding Modal ([`src/ui.html`](src/ui.html)):**
   - If `config.first_run` is true (or database file was freshly created):
     - Display a clean Apple/Arc-styled setup card.
     - Offer primary hotkey toggle: `Alt + Space` (Default) vs `Ctrl + Space`.
     - Confirm search libraries (Desktop, Documents, Downloads).
     - Provide a "Launch at Windows Startup" switch.
     - "Get Started" button dismisses onboarding and saves configuration.
3. **Footer Status Area during Indexing ([`src/ui.html`](src/ui.html)):**
   - Display a subtle status indicator in the search footer:
     - While indexing: `● Indexing 8,420 files...` with a subtle pulse animation.
     - When completed: `✓ Ready (42,150 files indexed)` which fades to normal footer after 3 seconds.
   - Search bar remains fully interactive and responsive to partial indexes throughout indexing.

---

## 3. Backward Compatibility & Migration Considerations

1. **SQLite Database Schema:**
   - Existing databases have `files`, `search_history`, and `query_frequencies`.
   - Adding a batch deletion helper and fixing `clear_learning_data()` requires no breaking schema migrations. All queries remain strictly compatible with existing `kelp.db` files.
2. **`config.json` Migration:**
   - `excluded_paths` already exists in `AppConfig` with `#[serde(default = "default_excluded_paths")]`.
   - Add `first_run: bool` with `#[serde(default = "default_false")]` so existing installations do not unexpectedly trigger the first-run onboarding wizard.
3. **Legacy Junction Exclusions:**
   - Retain Windows legacy junction point exclusions (`My Music`, `My Pictures`, `My Videos` inside `Documents`) to ensure Windows 7/10/11 compatibility without cycle loops.

---

## 4. Testing & Verification Strategy

### Automated Unit & Regression Tests
- `cargo test --lib`:
  - `test_clear_learning_data`: Verify clearing learning history properly purges both `search_history` and `query_frequencies` without SQLite errors.
  - `test_walkdir_entry_pruning`: Construct a mock filesystem containing `node_modules` with 1,000 subfiles; assert `WalkDir` with `filter_entry` inspects only 1 directory entry and prunes all 1,000 subfiles.
  - `test_batch_delete_stale_files`: Benchmark batch deletion of 2,500 file paths in SQLite in < 50ms.
  - `test_debounce_coalescing`: Feed 500 simulated rapid events into `FileWatcher` debounce channel; assert only 1 coalesced batch event is delivered.
  - `test_query_sequencing`: Send queries with IDs `[1, 2, 3]`; simulate out-of-order resolution `[2, 3, 1]`; assert results from query ID `1` are rejected.
  - `test_add_exclusion_purge`: Index mock folder; add exclusion; verify entries are purged from both `MemoryIndex` and SQLite immediately.

### Performance Benchmarks (Scale: 1k, 10k, 50k, 100k files)
- **Indexing Throughput:** Measure files indexed per second across dataset tiers.
- **Search Latency:** Measure end-to-end latency for:
  - Exact Match (e.g. `Visual Studio Code`)
  - Prefix Match (e.g. `vis`)
  - Substring Match (e.g. `code`)
  - Acronym Match (e.g. `vsc`)
  - Fuzzy / Miss Query (e.g. `nonexistentxyz`)
- **Memory Footprint:** Record Working Set and Private Bytes at 10k, 50k, and 100k files.

---

## 5. Measurable Acceptance Criteria

| Criteria | Pre-Fix Baseline | Target Requirement | Status Verification |
|---|---|---|---|
| **Cold Startup (First Launch, 50k files)** | 15s – 45s (Frozen/Unresponsive) | **< 600ms** to visible UI | Wall-clock timer to UI interactive |
| **Search Latency (10k files)** | 15ms – 45ms | **< 3ms** | Benchmark harness across all match types |
| **Search Latency (50k files)** | 120ms – 350ms | **< 12ms** | Benchmark harness across all match types |
| **Search Latency (100k files)** | 300ms – 850ms | **< 25ms** | Benchmark harness across all match types |
| **Out-of-Order Query Resolution** | Frequent race conditions | **0 race conditions** | Monotonic `query_id` assertion test |
| **File Watcher Storm (1,000 touches)** | 1,000 DB writes, 1,000 re-sorts | **1 batched write, 0 UI freeze** | File modification stress test |
| **Exclusion Addition Purge** | Non-functional | **< 50ms** instant memory removal | Exclusion add test in Settings |
| **Clear History Action** | Fails with SQLite error | **100% success** | IPC clear cache test |
| **Process RAM (100k files)** | ~140 MB – 220 MB | **< 55 MB** | Windows Performance Monitor (`WorkingSet`) |
