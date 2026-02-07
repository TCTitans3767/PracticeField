from enum import Enum
import asyncio
from bleak import BleakClient
import config as cfg

class LightColor(Enum):
    RED = "red"
    BLUE = "blue"
    WHITE = "white"
    FIELD_CONTROL = "field_control"
    OFF = "off"

class HubInterface:
    """
    Bluetooth LE Hub interface for ESP device.
    """

    def __init__(self, address: str):
        self.address = address
        self.client: BleakClient | None = None
        self.connected = False
        self.display = "----"
        self.light_color = LightColor.FIELD_CONTROL
        self.score_count = 0

    async def connect(self):
        """Connect to the ESP hub via BLE."""
        self.client = BleakClient(self.address)
        try:
            await self.client.connect()
            self.connected = await self.client.is_connected()
            return self.connected
        except Exception as e:
            print(f"Failed to connect: {e}")
            self.connected = False
            return False

    async def reset_hub(self):
        """Reset hub display, lights, and score."""
        if not self.connected:
            raise RuntimeError("Not connected")
        self.display = "----"
        self.light_color = LightColor.FIELD_CONTROL
        self.score_count = 0
        await self.set_display(self.display)
        await self.set_lights(self.light_color)

    async def get_score_count(self) -> int:
        """Read score count from hub."""
        if not self.connected:
            raise RuntimeError("Not connected")
        try:
            val = await self.client.read_gatt_char(cfg.SCORE_UUID)
            self.score_count = int.from_bytes(val, "little")
            return self.score_count
        except Exception as e:
            print(f"Failed to read score: {e}")
            return self.score_count

    async def set_display(self, scored_elements: str):
        """Write 4-character display to hub."""
        if not self.connected:
            raise RuntimeError("Not connected")
        if len(scored_elements) != 4:
            raise ValueError("Display must be exactly 4 characters")
        self.display = scored_elements
        try:
            await self.client.write_gatt_char(cfg.DISPLAY_UUID, scored_elements.encode())
        except Exception as e:
            print(f"Failed to write display: {e}")

    async def set_lights(self, color: LightColor):
        """Set lights color on hub."""
        if not self.connected:
            raise RuntimeError("Not connected")
        self.light_color = color
        try:
            await self.client.write_gatt_char(cfg.LIGHT_UUID, color.value.encode())
        except Exception as e:
            print(f"Failed to set lights: {e}")

    async def get_status(self) -> dict:
        """Return hub status."""
        score = await self.get_score_count()
        return {
            "connected": self.connected,
            "score_count": score,
            "display": self.display,
            "light_color": self.light_color.value,
        }

    async def disconnect(self):
        """Disconnect BLE hub."""
        if self.client:
            await self.client.disconnect()
            self.connected = False
