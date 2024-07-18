import threading
import queue
import yt_dlp
import os
import psutil
from concurrent.futures import ThreadPoolExecutor, wait, FIRST_COMPLETED

from helpers.notification_manager import NotificationManager


class DownloadManager(threading.Thread):
    def __init__(self, download_dir, download_archive, max_concurrent_downloads):
        super().__init__()
        self.notification_manager = NotificationManager()
        self.download_dir = download_dir
        self.download_archive = download_archive
        self.max_concurrent_downloads = max_concurrent_downloads
        self.download_queue = queue.Queue()
        self.status_queue = queue.Queue()
        self.stop_event = threading.Event()
        self.executor = ThreadPoolExecutor(max_workers=max_concurrent_downloads)
        self.current_futures = set()
        self.current_processes = {}  # Map of URL to process ID

    def run(self):
        while not self.stop_event.is_set():
            futures = set()
            while len(futures) < self.max_concurrent_downloads:
                try:
                    url = self.download_queue.get(block=False)
                    future = self.executor.submit(self.download_video, url)
                    futures.add(future)
                    self.current_futures.add(future)
                except queue.Empty:
                    break

            if futures:
                done, _ = wait(futures, return_when=FIRST_COMPLETED)
                for future in done:
                    self.current_futures.remove(future)
            else:
                # If no downloads were started, sleep briefly to avoid busy-waiting
                self.stop_event.wait(timeout=1)

    def download_video(self, url):
        ydl_opts = {
            'format': 'bestvideo*+bestaudio/best',
            'outtmpl': os.path.join(self.download_dir, '%(title)s - [%(id)s].%(ext)s'),
            'download_archive': self.download_archive,
            'progress_hooks': [self.progress_hook],
            'logger': self.YoutubeDLLogger(self),
        }

        try:
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                self.log('status', url, 'Downloading')
                self.current_processes[url] = os.getpid()
                info = ydl.extract_info(url, download=False)
                video_title = info.get('title', 'Unknown Title')
                ydl.download([url])
                del self.current_processes[url]
                if not self.stop_event.is_set():
                    self.log('status', url, 'Completed')
                    self.notification_manager.notify_download_complete(video_title)
        except Exception as e:
            if not self.stop_event.is_set():
                self.log('status', url, f'Error: {str(e)}')
                self.notification_manager.notify_download_error(url, str(e))
        finally:
            if url in self.current_processes:
                del self.current_processes[url]

    def progress_hook(self, d):
        if d['status'] == 'downloading' and not self.stop_event.is_set():
            progress = {
                'url': d['info_dict']['webpage_url'],
                'filename': d['filename'],
                'percent': d['_percent_str'],
                'total': d['_total_bytes_str'],
                'speed': d['_speed_str'],
                'eta': d['_eta_str'],
            }
            self.log('progress', progress)

    def log(self, message_type, *args):
        self.status_queue.put((message_type, *args))

    def add_download(self, url):
        self.download_queue.put(url)
        self.log('status', url, 'Queued')

    def stop(self):
        self.stop_event.set()
        for future in self.current_futures:
            future.cancel()
        self.executor.shutdown(wait=False)

        # Terminate all yt-dlp processes and update their status
        for url, pid in list(self.current_processes.items()):
            try:
                parent = psutil.Process(pid)
                children = parent.children(recursive=True)
                for child in children:
                    child.terminate()
                parent.terminate()
                self.log('status', url, 'Stopped')
            except psutil.NoSuchProcess:
                pass

        # Update status for queued downloads
        while not self.download_queue.empty():
            try:
                url = self.download_queue.get(block=False)
                self.log('status', url, 'Cancelled')
            except queue.Empty:
                break

        self.join(timeout=5)  # Wait for up to 5 seconds for the thread to finish

    class YoutubeDLLogger:
        def __init__(self, download_manager):
            self.download_manager = download_manager

        def debug(self, msg):
            if not self.download_manager.stop_event.is_set():
                self.download_manager.log('debug', msg)

        def warning(self, msg):
            if not self.download_manager.stop_event.is_set():
                self.download_manager.log('warning', msg)

        def error(self, msg):
            if not self.download_manager.stop_event.is_set():
                self.download_manager.log('error', msg)