import argparse

from vpn_manager import VPNManager
from config_manager import ConfigManager
from tui_manager import TUIManager
from logger import Logger
from auxiliary_features import AuxiliaryFeatures
from error_handler import AutoYTDLPErrorHandler
from performance_control import PerformanceControl
from notification_manager import NotificationManager


class AutoYTDLP:
    def __init__(self):
        self.config_manager = ConfigManager('config.toml')
        self.logger = Logger(self.config_manager.get('log_file', 'auto_ytdlp.log'))
        self.error_handler = AutoYTDLPErrorHandler(self.logger)
        self.vpn_manager = VPNManager(self.config_manager.get('vpn_settings', {}))
        self.notification_manager = NotificationManager()
        self.performance_control = PerformanceControl(
            max_concurrent_downloads=self.config_manager.get('max_concurrent_downloads', 3),
            bandwidth_limit=self.config_manager.get('bandwidth_limit')
        )
        self.auxiliary_features = AuxiliaryFeatures(self.performance_control.ydl_opts)
        self.tui_manager = TUIManager(self.start_downloads, self.stop_downloads)
        self.is_downloading = False

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
        urls = self.load_url_list(self.config_manager.get('url_list_file', 'list.txt'))
        for url in urls:
            self.performance_control.add_to_queue(url)
            self.tui_manager.add_download(url)

        results = self.performance_control.process_queue()
        for result in results:
            if result['status'] == 'success':
                self.tui_manager.update_download_status(result['url'], 'Completed')
                self.notification_manager.send_notification(f"Download complete: {result['url']}")
            elif result['status'] == 'error':
                self.tui_manager.update_download_status(result['url'], 'Failed')
                self.notification_manager.send_notification(f"Download failed: {result['url']}")
                self.logger.error(f"Download failed for {result['url']}: {result['error']}")
        self.is_downloading = False

    def stop_downloads(self):
        if not self.is_downloading:
            return
        self.performance_control.stop_queue()
        self.tui_manager.show_message("Downloads stopped.")
        self.is_downloading = False

    def run_cli(self):
        parser = argparse.ArgumentParser(description='Auto-YTDLP')
        parser.add_argument('--no-gui', action='store_true', help='Run in CLI mode without TUI')
        args = parser.parse_args()

        if args.no_gui:
            self.start_downloads()
        else:
            self.tui_manager.run()

    def run(self):
        try:
            self.vpn_manager.connect()
            self.run_cli()
        except Exception as e:
            self.error_handler.handle_error(e)
        finally:
            self.vpn_manager.disconnect()


if __name__ == "__main__":
    auto_ytdlp = AutoYTDLP()
    auto_ytdlp.run()