# src/ Architecture

## Threading Model

- **Main thread**: Runs TUI event loop (100ms tick rate)
- **AppState message processor**: Background thread spawned on `AppState::new()` - processes all state mutations via channel
- **Download controller**: Spawned when downloads start, manages N worker threads
- **Worker threads**: One per concurrent download, check shutdown flags to handle termination

## Core Files

### app_state.rs
Thread-safe state manager using message passing. All mutations go through `StateMessage` enum sent via `state.send()`. Never access internal mutexes directly from outside - use the public API methods.

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

## Data Flow

1. URLs loaded from `links.txt` → `StateMessage::LoadLinks` → queue
2. Worker pops URL with `state.pop_queue()` → spawns yt-dlp subprocess
3. Progress updates sent via `StateMessage::UpdateDownloadProgress`
4. On completion, URL removed from `links.txt` and `StateMessage::IncrementCompleted` sent
