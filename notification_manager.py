import platform
from plyer import notification


class NotificationManager:
    def __init__(self):
        self.system = platform.system()

    @staticmethod
    def send_notification(title, message):
        """
        Send a desktop notification.

        :param title: The title of the notification
        :param message: The body of the notification
        """
        try:
            notification.notify(
                title=title,
                message=message,
                app_name="Auto-YTDLP",
                timeout=10  # notification will disappear after 10 seconds
            )
        except Exception as e:
            print(f"Failed to send notification: {e}")

    def notify_download_complete(self, video_title):
        """
        Send a notification for a completed download.

        :param video_title: The title of the downloaded video
        """
        self.send_notification(
            title="Download Complete",
            message=f"The video '{video_title}' has finished downloading."
        )

    def notify_download_error(self, video_title, error_message):
        """
        Send a notification for a download error.

        :param video_title: The title of the video that encountered an error
        :param error_message: The error message
        """
        self.send_notification(
            title="Download Error",
            message=f"Error downloading '{video_title}': {error_message}"
        )

    def notify_vpn_switch(self, new_location):
        """
        Send a notification for a VPN server switch.

        :param new_location: The new VPN server location
        """
        self.send_notification(
            title="VPN Server Switched",
            message=f"Connected to new VPN server: {new_location}"
        )
