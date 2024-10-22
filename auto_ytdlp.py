import argparse
import sys
import threading

from helpers.vpn_manager import VPNManager
from helpers.config_manager import ConfigManager
from tui_manager import TUIManager
from helpers.logger import Logger
from helpers.error_handler import AutoYTDLPErrorHandler
from download_manager import DownloadManager


class AutoYTDLP:
    def __init__(self, use_tui=True, debug=False):
        self.config_manager = ConfigManager('./config.toml')
        self.logger = Logger(self.config_manager.get('general', 'log_file'))
        self.error_handler = AutoYTDLPErrorHandler(self.logger)
        self.vpn_manager = VPNManager(switch_after=self.config_manager.get('vpn', 'switch_after'))
        self.debug = debug

        self.initial_urls = self.load_url_list(self.config_manager.get('general', 'links_file'))

        self.download_manager = DownloadManager(
            download_dir=self.config_manager.get('general', 'download_dir'),
            download_archive=self.config_manager.get('yt_dlp', 'archive_file'),
            max_concurrent_downloads=self.config_manager.get('performance', 'max_concurrent_downloads')
        )

        self.tui_manager = TUIManager(
            self.start_downloads,
            self.stop_downloads,
            self.quit,
            self.download_manager,
            initial_urls=self.initial_urls,
            debug=self.debug,
            log_file=self.config_manager.get('general', 'log_file')
        ) if use_tui else None

    def load_url_list(self, file_path: str) -> list:
        try:
            with open(file_path, 'r') as f:
                return [line.strip() for line in f if line.strip()]
        except Exception as e:
            self.error_handler.handle_error(e)
            return []

    def start_downloads(self):
        self.download_manager.start()
        for url in self.initial_urls:
            self.download_manager.add_download(url)

    def stop_downloads(self):
        def stop_thread():
            try:
                self.logger.info("Stopping downloads...")
                self.download_manager.stop()
                self.logger.info("Downloads stopped successfully")
                if self.tui_manager:
                    self.tui_manager.update_output("All downloads have been stopped.")
            except Exception as e:
                self.error_handler.handle_error(e)
                self.logger.error(f"Failed to stop downloads: {str(e)}")
                if self.tui_manager:
                    self.tui_manager.update_output(f"Error stopping downloads: {str(e)}")

        threading.Thread(target=stop_thread).start()

    def quit(self):
        self.stop_downloads()
        self.vpn_manager.disconnect()
        sys.exit(0)

    def run(self):
        try:
            self.vpn_manager.connect()
            if self.tui_manager:
                self.tui_manager.run()
            else:
                self.start_downloads()
                self.download_manager.join()
        except Exception as e:
            self.error_handler.handle_error(e)
        finally:
            self.quit()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Auto-YTDLP')
    parser.add_argument('--no-gui', action='store_true', help='Run in CLI mode without TUI')
    parser.add_argument('--debug', action='store_true', help='Enable debug mode')
    args = parser.parse_args()

    auto_ytdlp = AutoYTDLP(use_tui=not args.no_gui, debug=args.debug)
    auto_ytdlp.run()
