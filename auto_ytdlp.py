import argparse
import asyncio

from vpn_manager import VPNManager
from config_manager import ConfigManager
from tui_manager import TUIManager
from logger import Logger
from auxiliary_features import AuxiliaryFeatures
from error_handler import AutoYTDLPErrorHandler
from performance_control import PerformanceControl
from notification_manager import NotificationManager


class AutoYTDLP:
    def __init__(self, use_tui=True):
        self.config_manager = ConfigManager('./config.toml')
        self.logger = Logger(self.config_manager.get('general', 'log_file'))
        self.error_handler = AutoYTDLPErrorHandler(self.logger)
        self.vpn_manager = VPNManager(switch_after=self.config_manager.get('vpn', 'switch_after'))
        self.notification_manager = NotificationManager()
        self.is_downloading = False
        self.max_retries = 1
        self.vpn_switch_needed = False
        self.download_count = 0
        initial_urls = self.load_url_list(self.config_manager.get('general', 'links_file'))
        self.tui_manager = TUIManager(self.start_downloads, self.stop_downloads, initial_urls) if use_tui else None

        self.performance_control = PerformanceControl(
            max_concurrent_downloads=self.config_manager.get('performance', 'max_concurrent_downloads'),
            tui_manager=self.tui_manager,
            download_dir=self.config_manager.get('general', 'download_dir'),
            download_archive=self.config_manager.get('yt_dlp', 'archive_file'),
        )

        self.auxiliary_features = AuxiliaryFeatures(self.performance_control.ydl_opts)

    def load_url_list(self, file_path: str) -> list:
        try:
            with open(file_path, 'r') as f:
                return [line.strip() for line in f if line.strip()]
        except Exception as e:
            self.error_handler.handle_error(e)
            return []

    def start_downloads(self):
        if self.is_downloading:
            return
        self.is_downloading = True
        urls = self.load_url_list(self.config_manager.get('general', 'links_file'))
        for url in urls:
            self.performance_control.add_to_queue(url)
            if self.tui_manager:
                self.tui_manager.add_download(url)

        try:
            self.performance_control.process_queue()
        except Exception as e:
            self.logger.exception(f"An unexpected error occurred during downloads: {str(e)}", exc_info=e)
            if self.tui_manager:
                self.tui_manager.show_output(f"An error occurred: {str(e)}")
        finally:
            self.is_downloading = False

    def stop_downloads(self):
        if not self.is_downloading:
            return
        self.performance_control.stop_queue()
        self.is_downloading = False

    def run_cli(self):
        if args.no_gui:
            self.start_downloads()
        else:
            self.tui_manager.run()

    def run(self):
        try:
            self.vpn_manager.connect()
            if self.tui_manager:
                self.tui_manager.run()
            else:
                self.start_downloads()
        except Exception as e:
            self.error_handler.handle_error(e)
        finally:
            self.vpn_manager.disconnect()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Auto-YTDLP')
    parser.add_argument('--no-gui', action='store_true', help='Run in CLI mode without TUI')
    args = parser.parse_args()

    auto_ytdlp = AutoYTDLP(use_tui=not args.no_gui)
    auto_ytdlp.run()
