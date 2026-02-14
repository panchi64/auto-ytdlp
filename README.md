# Auto-YTDLP

A concurrent video downloader built on [yt-dlp](https://github.com/yt-dlp/yt-dlp) with an interactive terminal interface. Queue up URLs, configure quality and format settings, and let it handle the rest.

<img width="1903" height="986" alt="Auto-YTDLP TUI Screenshot" src="https://github.com/user-attachments/assets/18cf03b0-369a-470e-b6da-fe230edde28b" />

## Features

- **Concurrent downloads** - Process multiple videos in parallel with configurable worker count
- **Interactive TUI** - Real-time progress bars, download speeds, ETAs, and log output
- **Headless mode** - Run without a UI for scripting, cron jobs, and automation
- **Queue management** - Reorder, filter, add, and remove URLs interactively
- **Settings panel** - Configure format, quality, subtitles, metadata, and more from within the TUI
- **Settings presets** - One-click profiles for common workflows (Best Quality, Audio Archive, Fast Download, Bandwidth Saver)
- **SponsorBlock** - Automatically remove sponsor segments from YouTube videos
- **Rate limiting** - Cap download speed per worker to manage bandwidth
- **Browser cookies** - Access age-restricted or authenticated content via browser cookie extraction
- **Network retry** - Automatically retry failed downloads with configurable delay
- **yt-dlp updates** - Check for and install yt-dlp updates from within the app
- **Download archive** - Skip previously downloaded videos across sessions
- **Clipboard integration** - Paste URLs directly from your clipboard into the queue
- **Desktop notifications** - Get notified when all downloads complete
- **Graceful shutdown** - Wait for active downloads to finish before exiting

## Requirements

- [yt-dlp](https://github.com/yt-dlp/yt-dlp/releases)
- [ffmpeg](https://www.ffmpeg.org/download.html)

Both must be installed and available in your `PATH`. The application validates this at startup.

## Installation

### Pre-built Binaries

Download the latest binary for your platform from the [Releases page](https://github.com/panchi64/auto-ytdlp/releases):

- **Linux:** `auto-ytdlp-<version>-linux`
- **macOS:** `auto-ytdlp-<version>-macos`
- **Windows:** `auto-ytdlp-<version>-windows.exe`

On macOS/Linux, make it executable and optionally move it to your `PATH`:

```bash
chmod +x auto-ytdlp-<version>-<platform>
sudo mv auto-ytdlp-<version>-<platform> /usr/local/bin/auto-ytdlp
```

> [!NOTE]
> On macOS, if you get an "app not opened" quarantine warning, run:
> ```bash
> xattr -dr com.apple.quarantine /path/to/auto-ytdlp-<version>-macos
> ```

### Cargo

```bash
cargo install auto-ytdlp-rs
```

### From Source

```bash
git clone https://github.com/panchi64/auto-ytdlp.git
cd auto-ytdlp
cargo build --release
# Binary is at target/release/auto-ytdlp
```

## Usage

### TUI Mode (default)

```bash
auto-ytdlp
```

This opens the interactive terminal interface. Add URLs to `links.txt` (one per line) in the working directory, then press **S** to start downloading.

### Automated Mode

```bash
auto-ytdlp --auto
```

Processes the queue without a UI and exits when complete. Useful for scripts and cron jobs.

### CLI Options

```
-a, --auto                         Run in automated mode without TUI
-c, --concurrent <CONCURRENT>      Max concurrent downloads [default: 4]
-d, --download-dir <DOWNLOAD_DIR>  Download directory [default: ./yt_dlp_downloads]
-f, --archive-file <ARCHIVE_FILE>  Archive file path [default: ./download_archive.txt]
-h, --help                         Print help
-V, --version                      Print version
```

**Example:**
```bash
auto-ytdlp --auto --concurrent 8 --download-dir ~/Videos
```

## TUI Controls

### Main Controls

| Key | Action |
|-----|--------|
| `S` | Start/Stop downloads |
| `P` | Pause/Resume active downloads |
| `Q` | Graceful quit (waits for active downloads to finish) |
| `Shift+Q` | Force quit (press twice within 2s to confirm) |
| `A` | Add URLs from clipboard |
| `R` | Reload queue from `links.txt` |
| `F` | Reload and sanitize `links.txt` (removes invalid URLs) |
| `E` | Enter queue edit mode |
| `/` | Filter/search queue |
| `U` | Update yt-dlp (blocked during active downloads) |
| `T` | Retry failed downloads |
| `X` | Dismiss stale download indicators |
| `F1` | Show help overlay |
| `F2` | Open settings panel |

### Queue Edit Mode (`E`)

| Key | Action |
|-----|--------|
| `Up/Down` | Navigate items |
| `K` | Move selected item up |
| `J` | Move selected item down |
| `D` / `Delete` | Remove selected item |
| `Esc` / `Enter` / `E` | Exit edit mode |

### Filter Mode (`/`)

| Key | Action |
|-----|--------|
| Type | Filter by substring (case-insensitive) |
| `Backspace` | Delete last character |
| `Enter` | Keep filter and exit filter mode |
| `Esc` | Clear filter and exit |

## Settings

Settings persist across sessions at `~/.config/auto-ytdlp/settings.json` (Linux/macOS) or `%APPDATA%\auto-ytdlp\settings.json` (Windows). Open the settings panel in the TUI with **F2**.

### Available Settings

| Setting | Default | Description |
|---------|---------|-------------|
| Format Preset | Best | Video quality: Best, Audio Only, 1080p, 720p, 480p, 360p |
| Output Format | Auto | Container: Auto, MP4, MKV, MP3, WEBM |
| Write Subtitles | Off | Download available subtitles (all languages) |
| Write Thumbnail | Off | Save video thumbnail as a separate image |
| Add Metadata | Off | Embed title, artist, date, etc. into the file |
| SponsorBlock | Off | Remove sponsor segments (YouTube) |
| Concurrent Downloads | 4 | Number of parallel download workers |
| Rate Limit | Unlimited | Download speed cap per worker (e.g., `1M`, `500K`) |
| Network Retry | Off | Automatically retry downloads that fail due to network errors |
| Retry Delay | 2s | Seconds to wait between retry attempts |
| Cookies from Browser | None | Browser to extract cookies from (Firefox, Chrome, Brave, etc.) |
| ASCII Indicators | Off | Use text `[OK]`/`[PAUSE]` instead of emoji indicators |
| Reset Stats on Batch | On | Reset counters each batch (Off = cumulative across batches) |
| Custom yt-dlp Args | Empty | Extra yt-dlp flags (validated to prevent conflicts) |

### Presets

Apply a full configuration profile from the settings panel:

- **Best Quality** - Highest quality with subtitles, thumbnails, and metadata
- **Audio Archive** - Audio-only MP3 with metadata (for music libraries)
- **Fast Download** - Best quality with 8 concurrent workers, no extras
- **Bandwidth Saver** - 480p, 2 workers, 2M rate limit, network retry with 5s delay

## File Management

| File | Purpose |
|------|---------|
| `links.txt` | Download queue — one URL per line. Created in the working directory. |
| `download_archive.txt` | Tracks downloaded video IDs to prevent re-downloads across sessions. |
| `~/.config/auto-ytdlp/settings.json` | Persistent settings (auto-created on first run). |

## Troubleshooting

1. **"yt-dlp/ffmpeg not found"** - Ensure both are installed and in your `PATH`
2. **Downloads fail immediately** - Check the logs panel for error details; verify URLs are valid and accessible
3. **Permission errors** - Ensure you have write access to the download directory
4. **Clipboard not working** - On Wayland (Linux), ensure `wl-copy`/`wl-paste` are available
5. **Authenticated content failing** - Set a browser for cookie extraction in settings (`F2`)

## FAQ

### Why does this exist?

I wrote this script originally in Python so that I didn't have to manually archive a massive list of YouTube university course videos (at the request of my professor). I built it around the yt-dlp repository because it extends the capabilities of manual downloads by incorporating a lot of QoL features. This script builds on top of that by adding multiple download multithreading and an intuitive TUI.

However I rewrote it in Rust, as a little practice for myself, since I've been trying to write more Rust recently.

It's not just for archiving course videos however! This script can handle all sorts of video-downloading tasks. Maybe you're a researcher collecting data, a content creator gathering inspiration, or just someone who likes to keep offline copies of their favorite online content. Whatever your reason, I hope my little repo helps.

And hey, if you think of some cool feature to add, the code's right there for you to tinker with!

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the [GNU GPLv3](LICENSE).

## Disclaimer

_This tool is for educational purposes only. **Please make sure you have the right to download any content before using this script.** I'm not responsible for any misuse of this software._
