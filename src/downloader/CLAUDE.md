# downloader/ Module

Handles the download orchestration and yt-dlp subprocess management.

## Files

### queue.rs
Controller that manages worker thread lifecycle. Key responsibilities:
- Creates workers on-demand when `process_queue()` is called
- Monitors queue and active downloads for completion
- Handles graceful shutdown vs force quit differently:
  - **Graceful**: Waits for all workers via `handle.join()`
  - **Force quit**: Exits controller loop immediately, workers self-terminate

Workers are wrapped in `catch_unwind` to recover from panics.

### worker.rs
Individual download logic. The `download_worker()` function:
1. Spawns `yt-dlp` subprocess with args from `common.rs`
2. Parses stdout line-by-line using `progress_parser.rs`
3. Throttles progress updates to 250ms intervals (reduces lock contention)
4. Handles network error retries based on settings
5. Removes successful URLs from `links.txt`

Use `should_abort()` helper to check force quit flag.

### progress_parser.rs
Parses yt-dlp output lines into structured `ProgressInfo`. Handles:
- Download progress (`[download] X.X%`)
- Post-processing messages
- Error detection
- Fragment progress for HLS/DASH streams

### common.rs
- `validate_dependencies()`: Checks yt-dlp and ffmpeg are in PATH
- `build_ytdlp_command_args()`: Constructs yt-dlp CLI arguments from settings

## Shutdown Behavior

Workers check these flags in their loop:
- `is_force_quit()`: Exit immediately, kill yt-dlp process
- `is_shutdown()`: Finish current download, then exit
- `is_paused()`: Sleep and continue checking
