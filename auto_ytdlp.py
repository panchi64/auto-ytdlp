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
        self.config_manager = ConfigManager('./config.toml')
        self.logger = Logger(self.config_manager.get('general', 'log_file'))
        self.error_handler = AutoYTDLPErrorHandler(self.logger)
        self.vpn_manager = VPNManager(switch_after=self.config_manager.get('vpn', 'switch_after'),
                                      speed_threshold=self.config_manager.get('vpn', 'speed_threshold'))
        self.tui_manager = TUIManager(self.start_downloads, self.stop_downloads)
        self.notification_manager = NotificationManager()
        self.performance_control = PerformanceControl(
            max_concurrent_downloads=self.config_manager.get('performance', 'max_concurrent_downloads'),
            bandwidth_limit=self.config_manager.get('performance', 'bandwidth_limit'),
            tui_manager=self.tui_manager,
            download_dir=self.config_manager.get('general', 'download_dir'),
        )
        self.auxiliary_features = AuxiliaryFeatures(self.performance_control.ydl_opts)
        self.tui_manager = TUIManager(self.start_downloads, self.stop_downloads)
        self.is_downloading = False
        self.max_retries = 1

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
            self.tui_manager.add_download(url)

        vpn_switched = False
        for attempt in range(self.max_retries + 1):
            results = self.performance_control.process_queue()
            retry_queue = []
            for result in results:
                current_speed = self.performance_control.get_current_speed()
                if not vpn_switched and self.vpn_manager.should_switch(current_speed):
                    self.vpn_manager.switch_server()
                    vpn_switched = True

                if result['status'] == 'success':
                    self.tui_manager.update_download_status(result['url'], 'Completed')
                    self.notification_manager.notify_download_complete(result['url'])
                elif result['status'] == 'error':
                    self.tui_manager.update_download_status(result['url'], 'Failed')
                    if attempt < self.max_retries:
                        retry_queue.append(result['url'])
                    else:
                        self.notification_manager.send_notification("Download failed",
                                                                    f"There was an error downloading: {result['url']}")
                        self.logger.error(f"Download failed for {result['url']}: {result['error']}")

            if not retry_queue:
                break

            self.performance_control.download_queue = retry_queue

        self.is_downloading = False

    def stop_downloads(self):
        if not self.is_downloading:
            return
        self.performance_control.stop_queue()
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
