import time
import webbrowser
from pathlib import Path
from enum import Enum
import pygame

MATCH_SOUNDS_DIR = Path(__file__).parent.parent / "MatchSounds"


class MatchSound(Enum):
    CHANGE_PHASE = "changePhase.wav"
    END_AUTON = "endAuto.wav"
    END_MATCH = "endMatch.wav"
    E_STOP = "eStop.wav"
    START_ENDGAME = "startEndGame.wav"
    START_MATCH = "startMatch.wav"
    START_TELEOP = "startTeleop.wav"


class AVController:
    """
    Handles AV functions for the field: opening browsers, playing sounds, etc.
    """

    def __init__(self):
        # Initialize the mixer
        pygame.mixer.init()
        self.current_channel: pygame.mixer.Channel | None = None
        self.sound_cache: dict[str, pygame.mixer.Sound] = {}

    def open_browser(self, url: str):
        """
        Open the given URL in the default browser.
        """
        webbrowser.open(url, new=1)

    def play_sound(self, sound: MatchSound):
        """
        Play a WAV file corresponding to the given MatchSound enum.
        Stops previous sound if still playing.
        """
        sound_path = MATCH_SOUNDS_DIR / sound.value
        if not sound_path.exists():
            raise FileNotFoundError(f"Sound file not found: {sound_path}")

        # Cache loaded sounds to avoid reloading every time
        if sound.value not in self.sound_cache:
            self.sound_cache[sound.value] = pygame.mixer.Sound(str(sound_path))
        wave_obj = self.sound_cache[sound.value]

        # Stop previous sound if still playing
        if self.current_channel and self.current_channel.get_busy():
            self.current_channel.stop()

        self.current_channel = wave_obj.play()

    def wait_done(self):
        """Block until the current sound finishes."""
        if self.current_channel:
            while self.current_channel.get_busy():
                pygame.time.wait(50)