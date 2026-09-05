# Changelog

All notable user-facing changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-09-05

### Added

- **Download directory setting.** The download folder is now editable from the
  settings panel (`F2`) and persisted like every other setting. Resolution order
  is `--download-dir`, then the setting, then the built-in `./yt_dlp_downloads`.
  A leading `~` is expanded by both sources. When the flag is passed it wins for
  that run and the panel says so instead of showing a directory it is overriding.
  Changing the directory mid-session applies without a restart. Presets and
  **Reset to Defaults** deliberately leave it alone.
- **Scrollable pending queue.** Scroll with the arrow keys, `PgUp`/`PgDn`,
  `Home`/`End`, or the mouse wheel over the panel. A scroll thumb appears on the
  panel border when the list overflows. In queue edit mode the wheel moves the
  selection rather than the viewport, so the highlighted row cannot scroll out of
  sight.
- **Newly added links flash** a fading highlight for a moment so they are easy to
  spot in a long queue.
- Uppercase `D` is now bound in queue edit mode, which the UI had always advertised.

### Changed

- The pending queue renders **newest-first**. Downloads still run in FIFO order,
  so workers continue to take the oldest link first.
- `-c/--concurrent` is now optional, so its default no longer overwrites the
  saved **Concurrent Downloads** setting on launch.
- Presets carry over the ASCII-indicator and cumulative-stats preferences, which
  are terminal and session preferences rather than download behaviour.

### Fixed

- **Downloads could be left stranded.** Workers now respawn when URLs arrive
  after the queue drained, instead of being stranded by a one-shot latch.
- **A graceful quit could exit mid-download.** The stop path no longer detaches
  the controller handle, so quit waits for the drain to finish.
- **The TUI could panic** on an out-of-range progress ratio, and the worker could
  panic parsing a progress line containing multi-byte characters.
- **A full stderr pipe from yt-dlp could hang a worker** and stall the whole
  batch. stderr is now drained on its own thread, and its last `ERROR` line
  reaches the log.
- Only `ERROR` lines decide whether a download is retryable; yt-dlp warnings were
  causing permanent failures to retry three times each.
- Stop/pause/resume, the `u`/`t` guards and the batch completion check were all
  inert because an internal flag was never set in production code. Guarded keys
  (`f`, `r`, `e`, `u`, `t`) now refuse for the whole drain, and a paused session
  counts as still in flight since pausing does not stop running subprocesses.
- A fast double-press of `S` can no longer spawn a duplicate controller, and
  restarting during a drain reports why instead of silently ignoring the key.
- `yt-dlp -U` is guarded against overlapping runs.
- A queue lock failure no longer reads as an empty queue and declares the batch
  complete.
- Progress no longer renders as `100% (5/3)` when a second, shorter batch starts,
  and cumulative stats can now actually reach 100%.
- A failed load of `links.txt` is reported instead of being counted as a success.
- Clipboard pastes are deduplicated within the pasted text, not just against the
  file, and an empty paste now reports back.
- `f` (load links) is guarded against running downloads, as `r` already was.
- Log auto-scroll measures word wrapping in display cells, matching the renderer.
- Stop no longer writes with `eprintln!` over the alternate screen.

### Internal

- `app_state`, `settings`, the TUI event loop and input handling, the settings
  menu, and the progress parser were each split into focused module trees, and
  inline test modules moved into sibling `tests.rs` files.
- Test suite grown from 429 to 462 tests.

### Compatibility

- Existing `~/.config/auto-ytdlp/settings.json` files load unchanged; the new
  `download_dir` field defaults to empty, meaning "use the flag or the default".

## [1.1.2] - 2026-02-13

### Added

- Rate limiting, SponsorBlock, browser cookie extraction, in-app yt-dlp updates,
  and automatic retry of failed downloads.
- Per-download progress bars with real-time percentage, speed, and ETA.
- Queue management and settings UX improvements.
- Desktop notifications and comprehensive TUI UX improvements.

### Changed

- Migrated from the `clipboard` crate to `arboard` for Wayland support.
- README rewritten with full feature documentation.

### Fixed

- The overall progress bar never advanced during downloads.
- Notifications looped infinitely and the terminal log display misbehaved.
- Race conditions, threading issues, and error handling throughout.
- Release workflow: correct binary names, consistent naming, checksums,
  auto-generated release notes, and the macOS Intel build.

## [1.0.51] - 2025-01-29

### Added

- Dependency checks for `yt-dlp` and `ffmpeg` before starting downloads.

## [1.0.5] - 2025-01-29

### Added

- Quality-of-life features and a basic test suite.

### Fixed

- The pending count showed `X/0` before downloads started.

## [1.0.2] - 2025-01-25

Initial published release: concurrent downloads driven by a terminal UI, a
`links.txt` queue, a download archive, clipboard URL import, progress bars, and
automated binary releases.

[1.2.0]: https://github.com/panchi64/auto-ytdlp/compare/v1.1.2...v1.2.0
[1.1.2]: https://github.com/panchi64/auto-ytdlp/compare/v1.0.51...v1.1.2
[1.0.51]: https://github.com/panchi64/auto-ytdlp/compare/v1.0.5...v1.0.51
[1.0.5]: https://github.com/panchi64/auto-ytdlp/compare/v1.0.2...v1.0.5
[1.0.2]: https://github.com/panchi64/auto-ytdlp/releases/tag/v1.0.2
