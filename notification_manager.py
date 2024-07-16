import platform
import subprocess
import win10toast

class NotificationManager:
    def __init__(self):
        self.system = platform.system()

    def send_notification(self, title, message):
        if self.system == "Windows":
            self._send_windows_notification(title, message)
        elif self.system == "Darwin":  # macOS
            self._send_macos_notification(title, message)
        elif self.system == "Linux":
            self._send_linux_notification(title, message)
        else:
            print(f"Unsupported OS for notifications: {self.system}")

    def _send_windows_notification(self, title, message):
        try:
            from win10toast import ToastNotifier
            toaster = ToastNotifier()
            toaster.show_toast(title, message, duration=5)
        except ImportError:
            print("win10toast not installed. Run 'pip install win10toast' to enable Windows notifications.")
            print(f"Notification: {title} - {message}")

    def _send_macos_notification(self, title, message):
        apple_script = f'display notification "{message}" with title "{title}"'
        subprocess.run(["osascript", "-e", apple_script])

    def _send_linux_notification(self, title, message):
        try:
            subprocess.run(["notify-send", title, message])
        except FileNotFoundError:
            print("notify-send not found. Make sure libnotify-bin is installed.")
            print(f"Notification: {title} - {message}")
