import os
import signal
import time
import concurrent.futures
from typing import List, Dict, Any, Callable
import yt_dlp


class PerformanceControl:
    def __init__(self,
                 max_concurrent_downloads: int = 3,
                 tui_manager=None,
                 download_dir: str = None,
                 download_archive: str = 'download_archive.txt'):
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

        elif d['status'] == 'finished':
            if self.tui_manager:
                self.tui_manager.show_output(f"Finished downloading {d['filename']}")

            # Reset for next download
            self.start_time = None
            self.downloaded_bytes = 0

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
                if self.tui_manager:
                    self.tui_manager.update_download_status(result['url'], result['status'])
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
