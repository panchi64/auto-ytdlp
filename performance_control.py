import os
import signal
import time
import concurrent.futures
from typing import List, Dict, Any
import yt_dlp
from threading import Lock

class PerformanceControl:
    def __init__(self,
                 max_concurrent_downloads: int = 3,
                 download_dir: str = None,
                 download_archive: str = 'download_archive.txt'):
        self.max_concurrent_downloads = max_concurrent_downloads
        self.download_queue = []
        self.download_archive = download_archive
        self.download_dir = download_dir or os.getcwd()
        self.ydl_opts = {
            'format': 'bestvideo*+bestaudio/best',
            'outtmpl': os.path.join(self.download_dir, '%(title)s - [%(id)s].%(ext)s'),
            'logger': self.YDLLogger(),
            'progress_hooks': [self.progress_hook],
            'download_archive': self.download_archive,
        }
        self.stop_flag = False
        self.current_ydl = None
        self.executor = None
        self.download_status = {}
        self.download_progress = {}
        self.status_lock = Lock()
        self.progress_lock = Lock()

    class YDLLogger:
        def debug(self, msg):
            pass

        def warning(self, msg):
            pass

        def error(self, msg):
            pass

    def add_to_queue(self, url: str):
        self.download_queue.append(url)
        with self.status_lock:
            self.download_status[url] = "Queued"

    def remove_from_queue(self, url: str):
        self.download_queue.remove(url)
        with self.status_lock:
            if url in self.download_status:
                del self.download_status[url]
        with self.progress_lock:
            if url in self.download_progress:
                del self.download_progress[url]

    def download_video(self, url: str) -> Dict[str, Any]:
        if self.stop_flag:
            return {"status": "stopped", "url": url}

        ydl_opts = self.ydl_opts.copy()

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            self.current_ydl = ydl
            try:
                with self.status_lock:
                    self.download_status[url] = "Downloading"
                ydl.download([url])
                with self.status_lock:
                    self.download_status[url] = "Completed"
                return {"status": "✅", "url": url}
            except yt_dlp.utils.DownloadError as e:
                if 'Cancelling download' in str(e):
                    with self.status_lock:
                        self.download_status[url] = "Cancelled"
                    return {"status": "cancelled", "url": url}
                with self.status_lock:
                    self.download_status[url] = "Error"
                return {"status": "error", "url": url, "error": str(e)}
            except Exception as e:
                with self.status_lock:
                    self.download_status[url] = "Error"
                return {"status": "error", "url": url, "error": str(e)}
            finally:
                self.current_ydl = None

    def progress_hook(self, d: Dict[str, Any]) -> None:
        if d['status'] == 'downloading':
            url = d.get('info_dict', {}).get('webpage_url', 'Unknown URL')
            progress = {
                'filename': d.get('filename', 'Unknown'),
                'percent': d.get('_percent_str', 'Unknown'),
                'total': d.get('_total_bytes_str', 'Unknown'),
                'speed': d.get('_speed_str', 'Unknown'),
                'eta': d.get('_eta_str', 'Unknown')
            }
            with self.progress_lock:
                self.download_progress[url] = progress
        elif d['status'] == 'finished':
            url = d.get('info_dict', {}).get('webpage_url', 'Unknown URL')
            with self.progress_lock:
                if url in self.download_progress:
                    del self.download_progress[url]

    def process_queue(self):
        self.stop_flag = False
        futures = []
        self.executor = concurrent.futures.ThreadPoolExecutor(max_workers=self.max_concurrent_downloads)

        for url in self.download_queue:
            if self.stop_flag:
                break
            future = self.executor.submit(self.download_video, url)
            futures.append(future)

        try:
            for future in concurrent.futures.as_completed(futures):
                if self.stop_flag:
                    break
                result = future.result()
        finally:
            self.stop_queue()

    def stop_queue(self):
        self.stop_flag = True
        if self.current_ydl:
            self.current_ydl.params['abort'] = True
        if self.executor:
            for pid in self.get_subprocess_pids():
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass  # Process already terminated
            self.executor.shutdown(wait=False)

    def get_subprocess_pids(self):
        if not self.executor:
            return []
        return [thread._thread.ident for thread in self.executor._threads]

    def get_download_status(self):
        with self.status_lock:
            return self.download_status.copy()

    def get_download_progress(self):
        with self.progress_lock:
            return self.download_progress.copy()