import os
from typing import Dict, Any

import toml


class ConfigManager:
    def __init__(self, config_file: str = "config.toml"):
        self.config_file = config_file
        self.config: Dict[str, Any] = self.get_default_config()
        self.load_config()
        self.ensure_download_directory()

    def load_config(self) -> None:
        try:
            with open(self.config_file, 'r') as f:
                loaded_config = toml.load(f)
                self.config = self.merge_configs(self.config, loaded_config)
        except FileNotFoundError:
            print(f"Config file {self.config_file} not found. Using default settings.")
        except toml.TomlDecodeError as e:
            print(f"Error parsing config file: {e}")
            print("Using default settings.")

    @staticmethod
    def get_default_config() -> Dict[str, Any]:
        return {
            "general": {
                "links_file": "links.txt",
                "download_dir": "./downloads",
                "log_file": "auto_ytdlp.logs"
            },
            "yt_dlp": {
                "archive_file": "download_archive.txt",
                "format": "bestvideo+bestaudio/best"
            },
            "performance": {
                "max_concurrent_downloads": 5,
                "bandwidth_limit": "5M"
            },
            "vpn": {
                "switch_after": 30,
                "speed_threshold": 500
            },
            "notifications": {
                "on_completion": True,
                "on_error": True
            }
        }

    def merge_configs(self, default: Dict[str, Any], loaded: Dict[str, Any]) -> Dict[str, Any]:
        for key, value in loaded.items():
            if isinstance(value, dict):
                default[key] = self.merge_configs(default.get(key, {}), value)
            else:
                default[key] = value
        return default

    def get(self, section: str, key: str, default: Any = None) -> Any:
        return self.config.get(section, {}).get(key, default)

    def set(self, section: str, key: str, value: Any) -> None:
        if section not in self.config:
            self.config[section] = {}
        self.config[section][key] = value

    def save_config(self) -> None:
        with open(self.config_file, 'w') as f:
            toml.dump(self.config, f)

    def ensure_download_directory(self) -> None:
        download_dir = self.get('general', 'download_dir')
        if download_dir:
            os.makedirs(download_dir, exist_ok=True)
            print(f"Ensured download directory exists: {download_dir}")
        else:
            print("Warning: No download directory specified in config.")
