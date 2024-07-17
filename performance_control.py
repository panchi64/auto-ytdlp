import os
import signal
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any, Callable
import yt_dlp


class PerformanceControl:
    def __init__(self,
                 max_concurrent_downloads: int = 3,
                 bandwidth_limit: str = None,
                 tui_manager=None,
                 download_dir: str = None):
        self.max_concurrent_downloads = max_concurrent_downloads
        self.bandwidth_limit = bandwidth_limit
        self.download_queue = []
        self.download_archive = set()
        self.download_dir = download_dir or os.getcwd()
        self.ydl_opts = {
            'format': 'bestaudio/best',
            'postprocessors': [{
                'key': 'FFmpegExtractAudio',
                'preferredcodec': 'mp3',
                'preferredquality': '192',
            }],
            'outtmpl': os.path.join(self.download_dir, '%(title)s.%(ext)s'),
            'logger': self.YDLLogger(tui_manager),
        }
        if bandwidth_limit:
            self.ydl_opts['ratelimit'] = bandwidth_limit
        self.stop_flag = False
        self.current_ydl = None
        self.executor = None
        self.progress_hooks: List[Callable] = [self.progress_hook]
        self.tui_manager = tui_manager

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

    def set_bandwidth_limit(self, limit: str):
        self.bandwidth_limit = limit
        self.ydl_opts['ratelimit'] = limit

    def set_format_preference(self, format_preference: str):
        self.ydl_opts['format'] = format_preference

    def set_download_path(self, path: str):
        self.ydl_opts['outtmpl'] = os.path.join(path, '%(title)s.%(ext)s')

    def load_download_archive(self, archive_file: str):
        if os.path.exists(archive_file):
            with open(archive_file, 'r') as f:
                self.download_archive = set(line.strip() for line in f)

    def save_download_archive(self, archive_file: str):
        with open(archive_file, 'w') as f:
            for item in self.download_archive:
                f.write(f"{item}\n")

    def download_video(self, url: str) -> Dict[str, Any]:
        if self.stop_flag:
            return {"status": "stopped", "url": url}

        ydl_opts = self.ydl_opts.copy()
        ydl_opts['progress_hooks'] = self.progress_hooks

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            self.current_ydl = ydl
            try:
                info = ydl.extract_info(url, download=False)
                video_id = info['id']
                if video_id in self.download_archive:
                    return {"status": "skipped", "url": url}

                ydl.download([url])
                self.download_archive.add(video_id)
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
        if self.stop_flag and d['status'] == 'downloading':
            raise yt_dlp.utils.DownloadError('Cancelling download')
        if self.tui_manager:
            if d['status'] == 'downloading':
                self.tui_manager.show_output(f"Downloading: {d['filename']} - {d.get('_percent_str', 'N/A')} complete")
            elif d['status'] == 'finished':
                self.tui_manager.show_output(f"Finished downloading {d['filename']}")

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
