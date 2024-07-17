# Auto-YTDLP

## Overview

I wrote this Python script so that I didn't have to manually archive a massive list of YouTube university course videos (at the request of my professor). I built it around the yt-dlp repository because it extends the capabilities of manual downloads by incorporating a lot of QoL features. This script builds on top of that by adding multiple download multithreading, VPN integration, and an intuitive CLI interface.

But it's not just for archiving course videos! This script can handle all sorts of video-downloading tasks. Maybe you're a researcher collecting data, a content creator gathering inspiration, or just someone who likes to keep offline copies of their favorite online content. Whatever your reason, I gotchu fam.

Here's what makes this script nice to have:

1. **Multi-video Downloading**: It can download multiple videos at once, so you're not sitting there, twiddling your thumbs waiting for one video to finish and then manually starting the next one.
2. **SHHH**: Download limits suck, that's why I added VPN support, which automatically switches IPs, so you can fly "under the radar" (for legal reasons that's a joke).
3. **Easy to Use**: The CLI interface is straightforward. You can see what's downloading, pause stuff, and remove things from the queue - all without breaking a sweat.
4. **Flexible**: I added a config file where you can tweak settings to your heart's content. Want to limit your bandwidth? No problem. Need to change how often the VPN switches? Easy-peasy.
5. **Hard shell**: It can handle long download sessions like a champ. If something goes wrong, it'll let you know, and you can gracefully shut it down without losing progress.
6. **Organized**: It keeps track of what you've already downloaded and pulls down video metadata too, so you're not left with a bunch of mystery files.

And hey, if you think of some cool feature to add, the code's right there for you to tinker with!

## Features

- Download videos from URLs listed in a text file
- CLI interface with progress display
- Multithreaded downloads for improved performance
- ExpressVPN integration with smart switching
- yt-dlp archive feature to avoid re-downloading
- Bandwidth throttling
- Verbose logging
- Parallel processing limit
- Desktop notification system
- Auto-update feature for yt-dlp
- Graceful shutdown
- Queue management through CLI GUI
- Metadata extraction (using yt-dlp's built-in functionality)
- All settings configurable via TOML file

## Requirements

- Python 3.7+
- yt-dlp
- ExpressVPN (installed and configured)
- Additional Python packages (listed in `requirements.txt`)

## Installation

1. Clone this repository:
   ```
   git clone https://github.com/panchi64/auto-ytdlp.git
   cd auto-ytdlp
   ```

2. Set up a virtual environment (recommended):
   ```
   python -m venv venv --prompt auto-ytdlp # Use python3 if python doesn't work
   ```

   Activate the virtual environment:
   - On Windows:
     ```
     venv\Scripts\activate
     ```
   - On macOS and Linux:
     ```
     source venv/bin/activate
     ```

3. Install required packages:
   ```
   pip install -r requirements.txt
   ```

4. Ensure ExpressVPN is installed and configured on your system by running `expressvpn status` in your terminal.

## Configuration

All settings are managed through a `config.toml` file. Create this file in the same directory as the script. Here's an example configuration with explanations:

```toml
[general]
links_file = "links.txt"
download_dir = "/path/to/downloads"
log_file = "auto_ytdlp.logs"

[yt_dlp]
archive_file = "download_archive.txt"  # File to store information about downloaded videos
format = "bestvideo+bestaudio/best"

[performance]
max_concurrent_downloads = 5
bandwidth_limit = "5M"  # 5 Mbps

[vpn]
switch_after = 30  # Switch VPN after every 30 downloads
speed_threshold = 500  # Switch if the speed drops below 500 KBps

[notifications]
on_completion = true
on_error = true
```
> [!NOTE]
> The `archive_file` setting in the `[yt_dlp]` section specifies the file where yt-dlp will store information about downloaded videos. This helps avoid re-downloading videos that have already been processed.

## Usage

1. Activate the virtual environment (if you haven't already):
   - On Windows:
     ```
     venv\Scripts\activate
     ```
   - On macOS and Linux:
     ```
     source venv/bin/activate
     ```
2. Prepare a text file (default: `links.txt`) with one video URL per line.

3. Run the script:
   ```
   python auto_ytdlp.py
   ```

4. Use the CLI interface to manage downloads, view progress, and control the script.

## CLI Interface Commands

- `start`: Begin downloading
- `pause`: Pause all downloads
- `resume`: Resume paused downloads
- `stop`: Stop all downloads and exit (graceful shutdown)
- `list`: Show current download queue
- `remove <id>`: Remove a specific download from the queue
- `info <id>`: Show detailed information about a specific download
- `update`: Check for and apply yt-dlp updates
- `no-gui`: No TUI, only CLI
- `help`: Show available commands

## Logging

Verbose logs are written to the specified log file (default: `auto_ytdlp.logs`). This includes download progress, errors, and system messages.

## Notifications

The script sends desktop notifications for completed downloads and errors. These notifications are OS-specific and will appear as standard system notifications if enabled.

## VPN Integration

The script integrates with ExpressVPN to protect your IP address. It will automatically switch connections based on the configured number of downloads or if a speed degradation is detected.

## Troubleshooting

1. Ensure ExpressVPN is properly installed and configured.
2. Check the log file for detailed error messages.
3. Verify that your `config.toml` file is correctly formatted and located in the script directory.
4. Make sure you have the necessary permissions for the download directory.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the GNU GPLv3 - look at the LICENSE file for details.

## Disclaimer

_This tool is for educational purposes only. **Please make sure you have the right to download any content before using this script.** I'm not responsible for any misuse of this software._
