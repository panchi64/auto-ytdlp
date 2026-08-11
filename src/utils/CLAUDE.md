# utils/ Module

Shared utilities for settings, file operations, dependencies, and display formatting.

## Files

### settings.rs
Persistent JSON settings at `~/.config/auto-ytdlp/settings.json`.

Key types:
- `Settings`: Main config struct with all options
- `FormatPreset`: Video quality presets (Best, AudioOnly, HD1080p, etc.)
- `OutputFormat`: Container format (Auto, MP4, MKV, MP3, WEBM)

`download_dir` is empty by default, meaning "use the CLI value or the built-in default" — resolve it through `Args::resolve_download_dir()`, never by reading the field directly. `expand_tilde()` handles `~` paths and returns `None` when there is no home directory, so callers fall back instead of creating a directory named `~`. `validate_download_dir()` only checks the path (it runs on the TUI thread); the directory is created in `main.rs` at startup and in `download_worker()` before each download.

In test builds `get_settings_path()` returns a per-thread temp file, so tests that exercise save/load never touch the developer's real `~/.config/auto-ytdlp/settings.json`.

Settings use atomic write (temp file + rename) to prevent corruption.

### file.rs
Operations on `links.txt`:
- `get_links_from_file()`: Read URLs from file
- `add_clipboard_links()`: Parse and add URLs from clipboard text
- `remove_link_from_file_sync()`: Remove completed URL (uses AppState file lock)
- `sanitize_links_file()`: Remove invalid URLs

**Important**: File operations acquire `state.acquire_file_lock()` to prevent race conditions when multiple workers complete simultaneously.

### dependencies.rs
Runtime validation that `yt-dlp` and `ffmpeg` exist in PATH.

### display.rs
URL formatting utilities:
- `truncate_url_for_display()`: Shortens URLs for TUI display, extracts YouTube video IDs
- `extract_youtube_id()`: Parses various YouTube URL formats

## File Lock Pattern

```rust
let _lock = state.acquire_file_lock()?;
// Safe to read/write links.txt here
// Lock released when _lock drops
```
