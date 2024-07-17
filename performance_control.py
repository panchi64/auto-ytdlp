import os
import signal
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any, Callable
import yt_dlp


class PerformanceControl:
    def __init__(self,
                 max_concurrent_downloads: int = 3,
                 tui_manager=None,
                 download_dir: str = None,
                 download_archive: str = 'download_archive.txt',):
        self.max_concurrent_downloads = max_concurrent_downloads
        self.download_queue = []
        self.download_archive = download_archive
        self.download_dir = download_dir or os.getcwd()
        self.ydl_opts = {
            'format': 'bestvideo*+bestaudio/best',
            'outtmpl': os.path.join(self.download_dir, '%(title)s - [%(id)s].%(ext)s'),
            'logger': self.YDLLogger(tui_manager),
            'download_archive': self.download_archive,
        }
        self.stop_flag = False
        self.current_ydl = None
        self.executor = None
        self.progress_hooks: List[Callable] = [self.progress_hook]
        self.tui_manager = tui_manager
        self.start_time = None
        self.downloaded_bytes = 0
        self.current_speed = 0.0

    class YDLLogger:
        def __init__(self, tui_manager):
            self.tui_manager = tui_manager

        def debug(self, msg):
            if self.tui_manager:
                self.tui_manager.show_output(f"[DEBUG] {msg}")

        def warning(self, msg):
            if self.tui_manager:
                self.tui_manager.show_output(f"[WARNING] {msg}")

        def error(self, msg):
            if self.tui_manager:
                self.tui_manager.show_output(f"[ERROR] {msg}")

    def add_to_queue(self, url: str):
        self.download_queue.append(url)

    def remove_from_queue(self, url: str):
        self.download_queue.remove(url)

    def download_video(self, url: str) -> Dict[str, Any]:
        if self.stop_flag:
            return {"status": "stopped", "url": url}

        ydl_opts = self.ydl_opts.copy()
        ydl_opts['progress_hooks'] = self.progress_hooks

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            self.current_ydl = ydl
            try:
                print(f'[INFO] Downloading {url}')
                ydl.download([url])
                return {"status": "success", "url": url}
            except yt_dlp.utils.DownloadError as e:
                if 'Cancelling download' in str(e):
                    return {"status": "cancelled", "url": url}
                return {"status": "error", "url": url, "error": str(e)}
            except Exception as e:
                return {"status": "error", "url": url, "error": str(e)}
            finally:
                self.current_ydl = None

    def progress_hook(self, d: Dict[str, Any]) -> None:
        if d['status'] == 'downloading':
            if self.start_time is None:
                self.start_time = time.time()

            self.downloaded_bytes = d['downloaded_bytes']
            elapsed_time = time.time() - self.start_time

            if elapsed_time > 0:
                self.current_speed = self.downloaded_bytes / elapsed_time / 1024  # Speed in KB/s

            if self.tui_manager:
                self.tui_manager.show_output(
                    f"Downloading: {d['filename']} - {d.get('_percent_str', 'N/A')} complete, Speed: {self.current_speed:.2f} KB/s")

        elif d['status'] == 'finished':
            if self.tui_manager:
                self.tui_manager.show_output(f"Finished downloading {d['filename']}")

            # Reset for next download
            self.start_time = None
            self.downloaded_bytes = 0
            self.current_speed = 0.0

    def get_current_speed(self) -> float:
        return float(self.current_speed)

    def process_queue(self):
        self.stop_flag = False
        results = []
        self.executor = ThreadPoolExecutor(max_workers=self.max_concurrent_downloads)
        try:
            future_to_url = {self.executor.submit(self.download_video, url): url for url in self.download_queue}
            for future in as_completed(future_to_url):
                url = future_to_url[future]
                try:
                    result = future.result()
                    results.append(result)
                except Exception as e:
                    results.append({"status": "error", "url": url, "error": str(e)})

                if self.stop_flag:
                    break
        finally:
            self.executor.shutdown(wait=False)
            self.executor = None

        return results

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

    def batch_process(self, batch_opts: List[Dict[str, Any]]):
        results = []
        for opts in batch_opts:
            if self.stop_flag:
                break
            temp_opts = self.ydl_opts.copy()
            temp_opts.update(opts.get('ydl_opts', {}))
            with yt_dlp.YoutubeDL(temp_opts) as ydl:
                try:
                    ydl.download(opts['urls'])
                    results.append({"status": "success", "urls": opts['urls']})
                except Exception as e:
                    results.append({"status": "error", "urls": opts['urls'], "error": str(e)})

        return results
