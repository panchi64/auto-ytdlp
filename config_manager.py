import toml
from typing import Any, Dict

class ConfigManager:
    def __init__(self, config_file: str = "config.toml"):
        self.config_file = config_file
        self.config: Dict[str, Any] = {}
        self.load_config()

    def load_config(self) -> None:
        """Load configuration from TOML file."""
        try:
            with open(self.config_file, 'r') as f:
                self.config = toml.load(f)
        except FileNotFoundError:
            print(f"Config file {self.config_file} not found. Using default settings.")
            self.config = self.get_default_config()
        except toml.TomlDecodeError as e:
            print(f"Error parsing config file: {e}")
            print("Using default settings.")
            self.config = self.get_default_config()

    def get_default_config(self) -> Dict[str, Any]:
        """Return default configuration settings."""
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

    def get(self, section: str, key: str, default: Any = None) -> Any:
        """Get a configuration value."""
        return self.config.get(section, {}).get(key, default)

    def set(self, section: str, key: str, value: Any) -> None:
        """Set a configuration value."""
        if section not in self.config:
            self.config[section] = {}
        self.config[section][key] = value

    def save_config(self) -> None:
        """Save the current configuration to the TOML file."""
        with open(self.config_file, 'w') as f:
            toml.dump(self.config, f)
