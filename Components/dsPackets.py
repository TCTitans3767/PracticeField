from enum import Enum
import datetime

from numpy import uint16, uint8

import config as cfg

class DsStation(Enum):
    STATION_1 = 0
    STATION_2 = 1
    STATION_3 = 2

class DsStationStatus(Enum):
    GOOD = 0
    BAD = 1
    WAITING = 2

class DsMode(Enum):
    TELEOP = 0
    TEST = 1
    AUTONOMOUS = 2

class TnLevel(Enum):
    TEST = 0
    PRACTICE = 1
    QUAL = 2
    PLAYOFF = 3


def create_udp_packet(packet_number: uint16,
                  is_estopped: bool,
                  is_astopped: bool,
                  is_enabled: bool,
                  mode: DsMode,
                  station: DsStation,
                  level: TnLevel,
                  match_number: uint16,
                  play_number: uint8,
                  remaining_time: uint16) -> bytes:
    packet = bytearray(22)

    packet[0] = (packet_number >> 8) & 0xFF
    packet[1] = packet_number & 0xFF

    # Comm version
    packet[2] = 0x00

    # Control/status byte
    packet[3] = 0x00
    if is_estopped: packet[3] |= 0b10000000
    if is_astopped: packet[3] |= 0b01000000
    if is_enabled: packet[3] |= 0b00000100
    packet[3] |= mode.value

    # /shrug
    packet[4] = 0x00

    # station
    packet[5] = station.value

    # level
    packet[6] = level.value

    # match number
    packet[7] = bytes(match_number)[1] if match_number > 0xFF else 0x00
    packet[8] = bytes(match_number)[0]

    # repeat number
    packet[9] = bytes(play_number)[0]

    # date
    now = datetime.datetime.now(datetime.timezone.utc)

    packet[10] = int(now.microsecond).to_bytes(4, byteorder="big")[0]
    packet[11] = int(now.microsecond).to_bytes(4, byteorder="big")[1]
    packet[12] = int(now.microsecond).to_bytes(4, byteorder="big")[2]
    packet[13] = int(now.microsecond).to_bytes(4, byteorder="big")[3]
    packet[14] = now.second.to_bytes(1, byteorder="big")[0]
    packet[15] = now.minute.to_bytes(1, byteorder="big")[0]
    packet[16] = now.hour.to_bytes(1, byteorder="big")[0]
    packet[17] = now.day.to_bytes(1, byteorder="big")[0]
    packet[18] = (now.month - 1).to_bytes(1, byteorder="big")[0]
    packet[19] = (now.year - 1900).to_bytes(1, byteorder="big")[0]

    # Remaining time
    packet[20] = bytes(int(remaining_time))[1] if remaining_time > 0xFF else 0x00
    packet[21] = bytes(int(remaining_time))[0]

    return bytes(packet)



def create_tcp_packet(tag) -> bytes:
    # size, uint16, including ID
    # ID, uint8, tag ID, get from tag
    # tag, length depends on tag, data got from tag
    packet = bytearray(3 + tag.get_length())
    length = tag.get_length() + 1
    packet[0] = bytes(int(length))[1] if length > 0xFF else 0x00
    packet[1] = bytes(int(length))[0]
    packet[2] = tag.id
    packet[3:] = tag.get_data()
    
    return bytes(packet)


class EventCodeTag:
    def __init__(self):
        self.id = 0x14

    def get_length(self) -> int:
        return len(cfg.EVENT_CODE)

    def get_data(self) -> bytes:
        return cfg.EVENT_CODE.encode('utf-8')
    
class StationInfoTag:
    def __init__(self, station: DsStation, station_status: DsStationStatus):
        self.id = 0x19
        self.station = station
        self.station_status = station_status

    def get_length(self) -> int:
        return 2

    def get_data(self) -> bytes:
        return bytes([self.station.value, self.station_status.value])
    
class GameDataTag:
    def __init__(self, game_data: str):
        self.id = 0x1A
        self.game_data = game_data
    
    def get_length(self) -> int:
        return len(self.game_data)
    
    def get_data(self) -> bytes:
        return self.game_data.encode('utf-8')