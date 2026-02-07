import socket
import subprocess
from typing import Tuple, Dict, Optional
import requests

import config as cfg


class DriverStationInterface:
    """
    Represents a single DS subnet/vlan/socket.
    """

    def __init__(self, name: str, vlan: int, xx: int, yy: int, fms_ip: int = 2):
        """
        name: 'ds1', 'ds2', or 'ds3'
        vlan: the vlan number for this DS (from config.py)
        xx: second octet of the DS IP address (e.g. 10.xx.yy.2)
        yy: third octet of the DS IP address (e.g. 10.xx.yy.2)
        fms_ip: the last octet of the FMS IP address (e.g. 10.xx.yy.fms_ip)
        """

        self.name = name
        self.vlan = vlan
        self.xx = xx
        self.yy = yy
        self.fms_ip = fms_ip

        self.interface = f"{cfg.HARDWARE_INTERFACE}.{vlan}"
        self.ip = f"10.{xx}.{yy}.{fms_ip}"
        self.socket = None

        self._setup_interface()
        self._create_socket()

    def _setup_interface(self):
        """Configure the VLAN interface IP (for linux)"""
        # Flush old IP
        subprocess.run(["sudo", "ip", "addr", "flush", "dev", self.interface], check=True)
        # Assign new IP
        subprocess.run(
            ["sudo", "ip", "addr", "add", f"{self.ip}/24", "dev", self.interface],
            check=True
        )
        # Ensure interface is up
        subprocess.run(["sudo", "ip", "link", "set", self.interface, "up"], check=True)

    def _create_socket(self):
        """Create UDP socket bound to the FMS IP of this VLAN"""
        self.socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.socket.bind((self.ip, 0))  # Bind to any available port

    def update_subnet(self, xx: int, yy: int):
        """Update the subnet IP and reconfigure the interface and socket"""
        self.xx = xx
        self.yy = yy
        self.ip = f"10.{xx}.{yy}.{self.fms_ip}"
        self._setup_interface()
        if self.socket:
            self.socket.close()
        self._create_socket()

    def send_udp(self, data: bytes, ds_port: int = cfg.DS_UDP_PORT):
        """Send UDP data to the DS IP on this VLAN"""
        ds_ip = f"10.{self.xx}.{self.yy}.5"
        self.socket.sendto(data, (ds_ip, ds_port))

    def send_tcp(self, data: bytes, ds_port: int = cfg.DS_TCP_PORT, keep_alive: bool = False):
        """
        Send TCP data to the DS IP on this VLAN.
        If keep_alive is True, the TCP socket will be reused.
        """
        ds_ip = f"10.{self.xx}.{self.yy}.5"

        if keep_alive:
            # Create socket if not exists
            if self.tcp_socket is None:
                self.tcp_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                self.tcp_socket.bind((self.ip, 0))  # Bind to VLAN IP
                self.tcp_socket.connect((ds_ip, ds_port))
            self.tcp_socket.sendall(data)
        else:
            # One-off connection
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.bind((self.ip, 0))
                s.connect((ds_ip, ds_port))
                s.sendall(data)


class RadioInterface:
    """
    Represents the FMS AP.
    """

    def __init__(self, vlan: int, xx: int, yy: int, ap_ip: int = 2, timeout: float = 2):
        """
        vlan: the vlan number for the radio (from config.py)
        xx: second octet of the radio IP address (e.g. 10.xx.yy.ap_ip)
        yy: third octet of the radio IP address (e.g. 10.xx.yy.ap_ip)
        ap_ip: the last octet of the radio IP address (e.g. 10.xx.yy.ap_ip)
        timeout: how long to wait for a response when checking connectivity
        """

        self.vlan = vlan
        self.xx = xx
        self.yy = yy
        self.ap_ip = ap_ip
        self.interface = f"{cfg.HARDWARE_INTERFACE}.{vlan}"  # VLAN interface name
        self.local_ip = f"10.{xx}.{yy}.{ap_ip}"
        self.timeout = timeout
        self.session: Optional[requests.Session] = None

        self._setup_vlan_interface()
        self._create_session()

    def _setup_vlan_interface(self):
        """Configure the VLAN interface IP on Linux."""
        subprocess.run(["sudo", "ip", "addr", "flush", "dev", self.interface], check=True)
        subprocess.run(["sudo", "ip", "addr", "add", f"{self.local_ip}/24", "dev", self.interface], check=True)
        subprocess.run(["sudo", "ip", "link", "set", self.interface, "up"], check=True)

    def _create_session(self):
        """Create a requests session."""
        if self.session:
            self.session.close()
        self.session = requests.Session()

    def update_subnet(self, xx: int, yy: int):
        """Update subnet IP dynamically and recreate session if needed."""
        self.xx = xx
        self.yy = yy
        self.local_ip = f"10.{xx}.{yy}.{self.ap_ip}"
        self._setup_vlan_interface()
        self._create_session()

    def http_get(self, path: str = "/", params: Optional[dict] = None) -> requests.Response:
        """Perform HTTP GET to the radio."""
        url = f"http://{self.local_ip}{path}"
        resp = self.session.get(url, params=params, timeout=self.timeout)
        resp.raise_for_status()
        return resp

    def http_post(self, path: str = "/", data: Optional[dict] = None) -> requests.Response:
        """Perform HTTP POST to the radio."""
        url = f"http://{self.local_ip}{path}"
        resp = self.session.post(url, data=data, timeout=self.timeout)
        resp.raise_for_status()
        return resp

    def close(self):
        """Close the HTTP session."""
        if self.session:
            self.session.close()
            self.session = None


class Network:
    """
    Holds driverstations and radio interfaces, manages their subnets, and provides methods to send UDP data.
    """

    def __init__(self):
        self._teams = [9991, 9992, 9993]
        self.driverstations: Dict[str, DriverStationInterface] = {
            "ds1": DriverStationInterface("ds1", cfg.DS_1_VLAN, self._xx_yy_from_team(self._teams[0])[0], self._xx_yy_from_team(self._teams[0])[1]),
            "ds2": DriverStationInterface("ds2", cfg.DS_2_VLAN, self._xx_yy_from_team(self._teams[1])[0], self._xx_yy_from_team(self._teams[1])[1]),
            "ds3": DriverStationInterface("ds3", cfg.DS_3_VLAN, self._xx_yy_from_team(self._teams[2])[0], self._xx_yy_from_team(self._teams[2])[1]),
        }
        self.radio = RadioInterface(cfg.DS_1_VLAN, cfg.RADIO_XX, cfg.RADIO_YY)
        
    def _xx_yy_from_team(self, team: int) -> Tuple[int, int]:
        """Calculate the xx and yy octets from the team number."""
        if team < 0 or team > 25599:
            raise ValueError("Team number must be 0-25599")
        
        s = str(team)
        if len(s) <= 2:       # 1-99
            xx = 0
            yy = int(s)
        elif len(s) == 3:     # 100-999
            xx = int(s[0])
            yy = int(s[1:])
        elif len(s) == 4:     # 1000-9999
            xx = int(s[:2])
            yy = int(s[2:])
        else:                 # 10000-25599
            xx = int(s[:3])
            yy = int(s[3:])
        
        return xx, yy

    def update_teams(self, teams: Tuple[int, int, int], wpa_keys: Tuple[str, str, str]):
        """Update team numbers and reconfigure driverstations and radio."""
        self._teams = teams
        for i, ds_name in enumerate(["ds1", "ds2", "ds3"]):
            xx, yy = self._xx_yy_from_team(teams[i])
            self.driverstations[ds_name].update_subnet(xx, yy)

        # TODO: self.radio.http_post(...) with new team info and WPA keys

    def send_udp_to_ds(self, ds_name: str, data: bytes):
        """Send UDP data to a specific driverstation."""
        if ds_name in self.driverstations:
            self.driverstations[ds_name].send_udp(data)
        else:
            raise ValueError(f"Invalid DS name: {ds_name}")
        
    def send_tcp_to_ds(self, ds_name: str, data: bytes, keep_alive: bool = False):
        """Send TCP data to a specific driverstation."""
        if ds_name in self.driverstations:
            self.driverstations[ds_name].send_tcp(data, keep_alive=keep_alive)
        else:
            raise ValueError(f"Invalid DS name: {ds_name}")
        
    def close(self):
        """Close all interfaces."""
        for ds in self.driverstations.values():
            if ds.socket:
                ds.socket.close()
        self.radio.close()

    def get_radio_status(self) -> Dict[str, str]:
        """Get status from the radio."""
        try:
            response = self.radio.http_get("/status")
            return response.json()
        except Exception as e:
            print(f"Error getting radio status: {e}")
            return {}