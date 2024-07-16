import sys
import logging
from urllib.error import URLError
from ssl import SSLError
from yt_dlp.utils import DownloadError, ExtractorError, GeoRestrictedError, YoutubeDLError

class AutoYTDLPErrorHandler:
    def __init__(self, logger):
        self.logger = logger

    def handle_file_error(self, error):
        if isinstance(error, FileNotFoundError):
            self.logger.error(f"File not found: {error.filename}")
        elif isinstance(error, PermissionError):
            self.logger.error(f"Permission denied: {error.filename}")
        else:
            self.logger.error(f"File operation error: {str(error)}")
        return False

    def handle_network_error(self, error):
        if isinstance(error, URLError):
            self.logger.error(f"Network error: {str(error)}")
        elif isinstance(error, SSLError):
            self.logger.error(f"SSL certificate error: {str(error)}")
        else:
            self.logger.error(f"Unspecified network error: {str(error)}")
        return False

    def handle_ytdlp_error(self, error):
        if isinstance(error, DownloadError):
            self.logger.error(f"Download failed: {str(error)}")
        elif isinstance(error, ExtractorError):
            self.logger.error(f"Extraction failed: {str(error)}")
        elif isinstance(error, GeoRestrictedError):
            self.logger.error(f"Content is geo-restricted: {str(error)}")
        else:
            self.logger.error(f"YouTube-DL error: {str(error)}")
        return False

    def handle_auth_error(self, error):
        self.logger.error(f"Authentication error: {str(error)}")
        return False

    def handle_parsing_error(self, error):
        self.logger.error(f"Parsing error: {str(error)}")
        return False

    def handle_external_tool_error(self, error):
        self.logger.error(f"External tool error: {str(error)}")
        return False

    def handle_system_error(self, error):
        self.logger.error(f"System error: {str(error)}")
        return False

    def handle_interrupt(self, error):
        self.logger.info("Operation interrupted by user.")
        return True

    def handle_update_error(self, error):
        self.logger.error(f"Update error: {str(error)}")
        return False

    def handle_config_error(self, error):
        self.logger.error(f"Configuration error: {str(error)}")
        return False

    def handle_playlist_error(self, error):
        self.logger.error(f"Playlist error: {str(error)}")
        return False

    def handle_format_selection_error(self, error):
        self.logger.error(f"Format selection error: {str(error)}")
        return False

    def handle_subtitle_error(self, error):
        self.logger.warning(f"Subtitle error: {str(error)}")
        return True  # Continue execution, just log the warning

    def handle_unexpected_error(self, error):
        self.logger.error(f"Unexpected error: {str(error)}")
        return False

    def handle_error(self, error):
        error_handlers = {
            OSError: self.handle_file_error,
            URLError: self.handle_network_error,
            SSLError: self.handle_network_error,
            YoutubeDLError: self.handle_ytdlp_error,
            ValueError: self.handle_parsing_error,
            KeyboardInterrupt: self.handle_interrupt,
        }

        handler = error_handlers.get(type(error), self.handle_unexpected_error)
        return handler(error)
