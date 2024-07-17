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
    def __init__(self, use_tui=True):
        self.config_manager = ConfigManager('./config.toml')
        self.logger = Logger(self.config_manager.get('general', 'log_file'))
        self.error_handler = AutoYTDLPErrorHandler(self.logger)
        self.vpn_manager = VPNManager(switch_after=self.config_manager.get('vpn', 'switch_after'),
                                      speed_threshold=self.config_manager.get('vpn', 'speed_threshold'))
        self.tui_manager = TUIManager(self.start_downloads, self.stop_downloads) if use_tui else None
        self.notification_manager = NotificationManager()
        self.performance_control = PerformanceControl(
            max_concurrent_downloads=self.config_manager.get('performance', 'max_concurrent_downloads'),
            tui_manager=self.tui_manager,
            download_dir=self.config_manager.get('general', 'download_dir'),
        )
        self.auxiliary_features = AuxiliaryFeatures(self.performance_control.ydl_opts)
        self.is_downloading = False
        self.max_retries = 1
        self.vpn_switch_needed = False
        self.download_count = 0

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
            while self.performance_control.download_queue:
                if self.vpn_switch_needed:
                    self.vpn_manager.switch_server()
                    self.vpn_switch_needed = False
                    if self.tui_manager:
                        self.tui_manager.show_output("VPN connection switched")

                current_url = self.performance_control.download_queue.pop(0)
                result = self.performance_control.download_video(current_url)

                current_speed = float(self.performance_control.get_current_speed())  # Ensure it's a float
                if self.vpn_manager.should_switch(current_speed):
                    self.vpn_switch_needed = True

                if result['status'] == 'success':
                    if self.tui_manager:
                        self.tui_manager.update_download_status(result['url'], 'Completed')
                    self.notification_manager.notify_download_complete(result['url'])
                    if self.tui_manager:
                        self.tui_manager.show_output(f"Download completed: {result['url']}")
                elif result['status'] == 'error':
                    if self.tui_manager:
                        self.tui_manager.update_download_status(result['url'], 'Failed')
                    self.notification_manager.send_notification("Download failed",
                                                                f"There was an error downloading: {result['url']}")
                    self.logger.exception(
                        f"Download failed for {result['url']}: {result.get('error', 'Unknown error')}")
                    if self.tui_manager:
                        self.tui_manager.show_output(
                            f"Download failed: {result['url']} - {result.get('error', 'Unknown error')}")
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
            self.run_cli()
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
