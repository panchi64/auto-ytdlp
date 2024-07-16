import os
import json
import subprocess
import sys
from typing import Dict, Any, List, Optional

import yt_dlp

class AuxiliaryFeatures:
    def __init__(self, ydl_opts: Dict[str, Any]):
        self.ydl_opts = ydl_opts

    def auto_update_yt_dlp(self) -> None:
        """Auto-update yt-dlp to the latest version."""
        subprocess.run([sys.executable, "-m", "pip", "install", "--upgrade", "yt-dlp"])
        print("yt-dlp has been updated to the latest version.")

    def extract_metadata(self, url: str) -> Dict[str, Any]:
        """Extract metadata from a video using yt-dlp's built-in functionality."""
        with yt_dlp.YoutubeDL(self.ydl_opts) as ydl:
            info = ydl.extract_info(url, download=False)
            return ydl.sanitize_info(info)

    def manage_download_archive(self, archive_file: str, video_id: str, action: str = "add") -> None:
        """Manage the download archive file."""
        if not os.path.exists(archive_file):
            open(archive_file, 'a').close()  # Create the file if it doesn't exist

        if action == "add":
            with open(archive_file, "a") as f:
                f.write(f"{video_id}\n")
        elif action == "remove":
            with open(archive_file, "r") as f:
                lines = f.readlines()
            with open(archive_file, "w") as f:
                f.writelines([line for line in lines if video_id not in line])

    def set_bandwidth_throttle(self, rate_limit: str) -> None:
        """Set bandwidth throttling for downloads."""
        self.ydl_opts['ratelimit'] = rate_limit

    def graceful_shutdown(self, ydl: yt_dlp.YoutubeDL) -> None:
        """Perform a graceful shutdown of the downloader."""
        ydl.interrupt()
        print("Gracefully shutting down. Cleaning up...")
        # Additional cleanup logic can be added here

    def utility_url_validation(self, url: str) -> bool:
        """Validate if the given URL is supported by yt-dlp."""
        extractors = yt_dlp.extractor.gen_extractors()
        for e in extractors:
            if e.suitable(url) and e.IE_NAME != 'generic':
                return True
        return False

    def process_url_file(self, file_path: str) -> List[str]:
        """Process a file containing URLs and return a list of valid URLs."""
        valid_urls = []
        with open(file_path, 'r') as f:
            for line in f:
                url = line.strip()
                if self.utility_url_validation(url):
                    valid_urls.append(url)
                else:
                    print(f"Warning: Unsupported URL - {url}")
        return valid_urls

    def download_with_progress(self, urls: List[str]) -> None:
        """Download videos with progress tracking."""
        def progress_hook(d):
            if d['status'] == 'downloading':
                print(f"\rDownloading {d['filename']}: {d['_percent_str']} of {d['_total_bytes_str']} at {d['_speed_str']}", end='')
            elif d['status'] == 'finished':
                print(f"\nFinished downloading {d['filename']}")

        ydl_opts = self.ydl_opts.copy()
        ydl_opts['progress_hooks'] = [progress_hook]

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            try:
                ydl.download(urls)
            except KeyboardInterrupt:
                self.graceful_shutdown(ydl)
