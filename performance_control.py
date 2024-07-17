import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import List, Dict, Any
import yt_dlp


class PerformanceControl:
    def __init__(self, max_concurrent_downloads: int = 3, bandwidth_limit: str = None):
        self.max_concurrent_downloads = max_concurrent_downloads
        self.bandwidth_limit = bandwidth_limit
        self.download_queue = []
        self.download_archive = set()
        self.ydl_opts = {
            'format': 'bestaudio/best',
            'postprocessors': [{
                'key': 'FFmpegExtractAudio',
                'preferredcodec': 'mp3',
                'preferredquality': '192',
            }],
            'outtmpl': '%(title)s.%(ext)s',
        }
        if bandwidth_limit:
            self.ydl_opts['ratelimit'] = bandwidth_limit
        self.stop_flag = False

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

        with yt_dlp.YoutubeDL(self.ydl_opts) as ydl:
            try:
                info = ydl.extract_info(url, download=False)
                video_id = info['id']
                if video_id in self.download_archive:
                    return {"status": "skipped", "url": url}

                ydl.download([url])
                self.download_archive.add(video_id)
                return {"status": "success", "url": url}
            except Exception as e:
                return {"status": "error", "url": url, "error": str(e)}

    def process_queue(self):
        self.stop_flag = False
        results = []
        with ThreadPoolExecutor(max_workers=self.max_concurrent_downloads) as executor:
            future_to_url = {executor.submit(self.download_video, url): url for url in self.download_queue}
            for future in as_completed(future_to_url):
                url = future_to_url[future]
                try:
                    result = future.result()
                    results.append(result)
                except Exception as e:
                    results.append({"status": "error", "url": url, "error": str(e)})

                if self.stop_flag:
                    break

        return results

    def stop_queue(self):
        self.stop_flag = True
        # You might want to add more logic here to cancel ongoing downloads
        # This might involve modifying yt-dlp options or sending signals to running processes

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
