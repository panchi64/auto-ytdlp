# Auto-YTDLP

<img width="1706" alt="image" src="https://github.com/user-attachments/assets/78f6ed47-1158-4c5e-ab69-9f6a57aff702">

## Overview

I wrote this script originally in Python so that I didn't have to manually archive a massive list of YouTube university course videos (at the request of my professor). I built it around the yt-dlp repository because it extends the capabilities of manual downloads by incorporating a lot of QoL features. This script builds on top of that by adding multiple download multithreading, VPN integration, and an intuitive TUI.

However I rewrote it in Rust as a little practice for myself, since I've been trying to write more Rust recently.

It's not just for archiving course videos however! This script can handle all sorts of video-downloading tasks. Maybe you're a researcher collecting data, a content creator gathering inspiration, or just someone who likes to keep offline copies of their favorite online content. Whatever your reason, I gotchu fam.

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
- Additional dependencies will be handled by Cargo

## Installation

1. Clone this repository:
   ```
   git clone https://github.com/panchi64/auto-ytdlp.git
   cd auto-ytdlp
   ```

2. Build the project:
   ```
   cargo build --release
   ```

3. Run the application:
   ```
   cargo run --release
   ```   
> [!WARNING]
> You need FFMPEG and yt-dlp installed in your system for the script to work appropriately.

## Usage

The application can be run in two modes:

### TUI Mode (Default):
```
./auto-ytdlp-rs
```

#### Interface Controls
- `S`: Start/Stop downloads
- `P`: Pause active downloads
- `R`: Resume paused downloads
- `A`: Add URLs from clipboard
- `Q`: Graceful shutdown
- `Shift+Q`: Force quit

> [!NOTE]
> All quit options will wait for the currently active downloads to finish, even the **Force Quit**

### Automated Mode (no TUI):
```
./auto-ytdlp-rust --auto
```

#### Command Line options
```
-a, --auto              Run in automated mode without TUI
-c, --concurrent <N>    Set maximum concurrent downloads (default: 4)
-d, --download-dir <P>  Specify download directory (default: "./yt_dlp_downloads")
-h, --archive-file <P>  Specify archive file location (default: "download_archive.txt")
```

## File Management
The application handles several important files:

- `links.txt`: Contains your download queue
- `download_archive.txt`: Tracks completed downloads
- _Download directory_: Where your content gets saved

## Troubleshooting

1. Ensure yt-dlp is properly installed and in your PATH
2. Check the logs panel for detailed error messages
3. Verify your URLs are valid and accessible
4. Make sure you have write permissions in the download directory

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the GNU GPLv3 - look at the LICENSE file for details.

## Disclaimer

_This tool is for educational purposes only. **Please make sure you have the right to download any content before using this script.** I'm not responsible for any misuse of this software._
