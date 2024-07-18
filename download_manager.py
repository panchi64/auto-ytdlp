import threading
import queue
import yt_dlp
import os

class DownloadManager(threading.Thread):
    def __init__(self, download_dir, download_archive, max_concurrent_downloads):
        super().__init__()
        self.download_dir = download_dir
        self.download_archive = download_archive
        self.max_concurrent_downloads = max_concurrent_downloads
        self.download_queue = queue.Queue()
        self.status_queue = queue.Queue()
        self.stop_event = threading.Event()

    def run(self):
        while not self.stop_event.is_set():
            try:
                url = self.download_queue.get(timeout=1)
                self.download_video(url)
            except queue.Empty:
                continue

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
                ydl.download([url])
                self.log('status', url, 'Completed')
        except Exception as e:
            self.log('status', url, f'Error: {str(e)}')

    def progress_hook(self, d):
        if d['status'] == 'downloading':
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

    class YoutubeDLLogger:
        def __init__(self, download_manager):
            self.download_manager = download_manager

        def debug(self, msg):
            self.download_manager.log('debug', msg)

        def warning(self, msg):
            self.download_manager.log('warning', msg)

        def error(self, msg):
            self.download_manager.log('error', msg)