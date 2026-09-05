# src/ Architecture

## Threading Model

- **Main thread**: Runs TUI event loop (100ms tick rate)
- **AppState message processor**: Background thread spawned on `AppState::new()` - processes all state mutations via channel
- **Download controller**: Spawned when downloads start, manages N worker threads
- **Worker threads**: One per concurrent download, check shutdown flags to handle termination

## Module Layout Conventions

Modules use the Rust 2018 style: a module root stays as `foo.rs` with its submodules
in a sibling `foo/` directory - there are no `mod.rs` files except where a directory
has no natural root (`ui/tui/render/`, `ui/settings_menu/`).

**Tests live beside the code they exercise.** Every module with unit tests declares
`#[cfg(test)] mod tests;` and the tests sit in `foo/tests.rs`. Never inline a
`mod tests { .. }` block - the repo was split this way precisely because inline tests
had grown to outweigh the production code.

Rust privacy is descendant-scoped, so a private field in `foo.rs` is still visible in
`foo/bar.rs`. Prefer keeping fields private and putting the code that touches them in a
child module over widening visibility. Where a sibling genuinely needs access, use
`pub(in crate::path::to::module)` rather than `pub(crate)`.

## Core Files

### app_state/
Thread-safe state manager using message passing. All mutations go through `StateMessage` enum sent via `state.send()`. Never access internal mutexes directly from outside - use the public API methods.

`app_state.rs` holds only the `AppState` struct, its private state structs and `new()`.
The rest is split: `message.rs` (`StateMessage`), `progress.rs` (`DownloadProgress`),
`snapshot.rs` (`UiSnapshot` + `get_ui_snapshot`), `dispatch.rs` (the message loop and
its handlers), and `api.rs` plus `api/{queue,flags,logs}.rs` for the public API.

Several methods deliberately span concerns because their **lock ordering is
load-bearing** (`remove_from_queue` takes queues then stats; `update_progress` takes
stats then flags; `reset_for_new_run` touches nearly everything). Move such a function
whole or not at all - decomposing it per-field reintroduces deadlock risk.

Key pattern:
```rust
state.send(StateMessage::SetPaused(true))?;  // Async mutation
let is_paused = state.is_paused()?;          // Sync read
```

The `UiSnapshot` struct captures all UI state in one lock acquisition per frame.

### errors.rs
Uses `thiserror`. The `Result<T>` type alias wraps `AppError`. Mutex poison errors convert to `AppError::Lock`.

### args.rs
CLI argument parsing with `clap`. Defines `-c` (concurrent), `-d` (download dir), `-f` (archive file), `--auto` (no TUI).

`-d` is optional; `Args::resolve_download_dir()` falls back to the `download_dir` setting and then to `DEFAULT_DOWNLOAD_DIR`. The output template is rebuilt per download so directory changes made in the settings menu apply without a restart.

## Data Flow

1. URLs loaded from `links.txt` → `StateMessage::LoadLinks` → queue
2. Worker pops URL with `state.pop_queue()` → spawns yt-dlp subprocess
3. Progress updates sent via `StateMessage::UpdateDownloadProgress`
4. On completion, URL removed from `links.txt` and `StateMessage::IncrementCompleted` sent
