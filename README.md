# Auto-YTDLP

![image](https://github.com/user-attachments/assets/d21d3df2-9905-48fc-b058-3b06ae91f449)

## Overview

I wrote this script originally in Python so that I didn't have to manually archive a massive list of YouTube university course videos (at the request of my professor). I built it around the yt-dlp repository because it extends the capabilities of manual downloads by incorporating a lot of QoL features. This script builds on top of that by adding multiple download multithreading and an intuitive TUI.

However I rewrote it in Rust, as a little practice for myself, since I've been trying to write more Rust recently.

It's not just for archiving course videos however! This script can handle all sorts of video-downloading tasks. Maybe you're a researcher collecting data, a content creator gathering inspiration, or just someone who likes to keep offline copies of their favorite online content. Whatever your reason, I hope my little repo helps.

Here's what makes this script nice to have:

1. **Multi-video Downloading**: Downloads multiple videos concurrently, making efficient use of your bandwidth and saving time.
2. **Clean Interface**: The TUI shows you exactly what's happening - active downloads, queue status, and detailed logs all in one view.
3. **Easy to Use**: Simple keyboard controls let you manage downloads, pause/resume operations, and add new URLs without hassle.
4. **Flexible**: Configure concurrent download limits, download directories, and archive locations to suit your needs.
5. **Hard Shell**: Built in Rust for rock-solid stability. Handles long download sessions reliably and shuts down gracefully when needed.
6. **Organized**: Keeps track of downloaded videos and maintains clean logs, so you always know what's happening.

And hey, if you think of some cool feature to add, the code's right there for you to tinker with!

## Features

- Download videos from URLs listed in a text file
- Terminal User Interface (TUI)
- Multithreaded downloads for improved performance
- yt-dlp archive feature to avoid re-downloading
- Verbose logging
- Parallel processing limit
- Desktop notification system
- Graceful shutdown
- Metadata extraction (using yt-dlp's built-in functionality)
- Clipboard integration for easy URL addition
- Progress tracking with visual feedback

## Requirements

- Rust (latest stable version)
- yt-dlp
- ffmpeg (yt-dlp requires it as an internal dependency)
- Additional dependencies will be handled by Cargo

## Installation

### Using [Pre-built Binaries](https://github.com/panchi64/auto-ytdlp/releases/new)

1. Go to the [Releases page](https://github.com/panchi64/auto-ytdlp/releases/new)
2. Download the appropriate binary for your system:

   - Windows: `auto-ytdlp-[version]-windows.exe`
   - macOS: `auto-ytdlp-[version]-macos`
   - Linux: `auto-ytdlp-[version]-linux`


3. Make the binary executable (macOS/Linux only):
   ```bash
   chmod +x auto-ytdlp-[version]-[platform]
   ```

4. Optional: Move the binary to a directory in your `PATH`:

   - Windows: Move to `C:\Windows\` or add the binary location to your `PATH`
   - macOS/Linux:
      ```bash
      sudo mv auto-ytdlp-[version]-[platform] /usr/local/bin/auto-ytdlp
      ```

### Building from source

1. Clone this repository:
   ```bash
   git clone https://github.com/panchi64/auto-ytdlp.git
   cd auto-ytdlp
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run the application:
   ```bash
   cargo run --release
   ```   
> [!WARNING]
> You need FFMPEG and yt-dlp installed in your system for the script to work appropriately.

## Usage

The application can be run in two modes:

### TUI Mode (Default):
```bash
# Using pre-built binary
./auto-ytdlp

# Or if installed to PATH
auto-ytdlp
```

#### Interface Controls
- `S`: Start/Stop downloads
- `P`: Pause active downloads
- `R`: Refresh downloads from links list file
- `A`: Add URLs from clipboard
- `Q`: Graceful shutdown
- `Shift+Q`: Force quit

> [!NOTE]
> All quit options will wait for the currently active downloads to finish, even the **Force Quit**

### Automated Mode (no TUI):
```bash
# Using pre-built binary
./auto-ytdlp --auto

# Or if installed to PATH
auto-ytdlp --auto
```

#### Command Line options
```
-a, --auto                         Run in automated mode without TUI
-c, --concurrent <CONCURRENT>      Max concurrent downloads [default: 4]
-d, --download-dir <DOWNLOAD_DIR>  Download directory [default: ./yt_dlp_downloads]
-f, --archive-file <ARCHIVE_FILE>  Archive file path [default: ./download_archive.txt]
-h, --help                         Print help
-V, --version                      Print version
```

## File Management
The application handles several important files:

- `links.txt`: Contains your download queue
- `download_archive.txt`: Tracks completed downloads
- _Download directory_: Where your content gets saved

## Troubleshooting

1. Ensure yt-dlp is properly installed and in your `PATH`
2. Check the logs panel for detailed error messages
3. Verify your URLs are valid and accessible
4. Make sure you have write permissions in the download directory

If you get the "auto-ytdlp-[version]-macos not opened" message on Apple devices. Use the following command to remove it from quarantine:
```
xattr -dr com.apple.quarantine <path to file>/auto-ytdlp-[version]-macos
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the GNU GPLv3 - look at the LICENSE file for details.

## Disclaimer

_This tool is for educational purposes only. **Please make sure you have the right to download any content before using this script.** I'm not responsible for any misuse of this software._
