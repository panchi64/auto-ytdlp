import subprocess
import time
from typing import Tuple


class VPNManager:
    def __init__(self, switch_after: int = 30, speed_threshold: int = 500):
        self.switch_after = switch_after
        self.speed_threshold = float(speed_threshold)
        self.download_count = 0

    @staticmethod
    def connect() -> bool:
        """Connect to ExpressVPN."""
        try:
            result = subprocess.run(["expressvpn", "connect"], capture_output=True, text=True, check=True)
            return "Connected to" in result.stdout
        except subprocess.CalledProcessError:
            print("Failed to connect to ExpressVPN")
            return False

    @staticmethod
    def disconnect() -> bool:
        """Disconnect from ExpressVPN."""
        try:
            result = subprocess.run(["expressvpn", "disconnect"], capture_output=True, text=True, check=True)
            return "Disconnected" in result.stdout
        except subprocess.CalledProcessError:
            print("Failed to disconnect from ExpressVPN")
            return False

    def switch_server(self) -> bool:
        """Switch to a different VPN server."""
        if self.disconnect():
            time.sleep(2)  # Wait for disconnection to complete
            return self.connect()
        return False

    @staticmethod
    def check_connection() -> Tuple[bool, str]:
        """Check the current VPN connection status."""
        try:
            result = subprocess.run(["expressvpn", "status"], capture_output=True, text=True, check=True)
            connected = "Connected to" in result.stdout
            location = result.stdout.split("Connected to")[1].strip() if connected else "Not connected"
            return connected, location
        except subprocess.CalledProcessError:
            return False, "Unknown"

    def should_switch(self, current_speed: float) -> bool:
        """Determine if we should switch VPN servers."""
        self.download_count += 1
        if self.download_count >= self.switch_after:
            self.download_count = 0
            return True
        if current_speed < self.speed_threshold:
            return True
        return False
