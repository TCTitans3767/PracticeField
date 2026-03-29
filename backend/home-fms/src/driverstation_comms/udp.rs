use std::time::Duration;

use tokio::net::UdpSocket;

use crate::driverstation_comms::driverstation_connection;

pub struct FMSUDPPacket {
}

pub struct DSUDPData {
    team_number: u16,
    control_mode: [ControlMode; 2],
    alliance_station: u8,
    is_e_stopped: bool,
    is_ds_alive: bool,
}

pub enum ControlMode {
    EStop,
    AStop,
    Enabled,
    Disabled,
    Teleop,
    Autonomous
}

const RED_1: u8 = 0;
const RED_2: u8 = 1;
const RED_3: u8 = 2;
const BLUE_1: u8 = 3;
const BLUE_2: u8 = 4;
const BLUE_3: u8 = 5;

const UDP_ADDRESS = "10.0.100.5:1120";

async fn driverstation_udp_conn(team_number: u16, udp_conn: &DSUDPData) {
    let mut udp_connection = DSUDPData {
        team_number,
        control_mode: [ControlMode::Teleop, ControlMode::Disabled],
        alliance_station: BLUE_1,
        is_e_stopped: false,
        is_ds_alive: true
    };
    let socket = UdpSocket::bind()
    loop {
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
