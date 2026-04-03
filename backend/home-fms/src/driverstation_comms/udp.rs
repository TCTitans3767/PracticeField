use std::time::Duration;

use tokio::net::UdpSocket;

use crate::driverstation_comms::driverstation_connection::DriverstationConnection;

pub struct FMSUDPPacket {}

#[derive(Clone)]
pub struct DSUDPData {
    pub team_number: u16,
    pub control_mode: [ControlMode; 2],
    pub alliance_station: u8,
}

impl DSUDPData {
    pub fn new(team_number: u16) -> Self {
        Self {
            team_number,
            control_mode: [ControlMode::Teleop, ControlMode::Disabled],
            alliance_station: BLUE_1,
        }
    }
}

#[derive(Clone)]
pub enum ControlMode {
    EStop,
    AStop,
    Enabled,
    Disabled,
    Teleop,
    Autonomous,
    Test,
}

pub const RED_1: u8 = 0;
pub const RED_2: u8 = 1;
pub const RED_3: u8 = 2;
pub const BLUE_1: u8 = 3;
pub const BLUE_2: u8 = 4;
pub const BLUE_3: u8 = 5;

pub fn create_udp_packet(data: &DriverstationConnection) -> Vec<u8> {
    let mut packet_data: Vec<u8> = Vec::new();

    packet_data.push(0x0); // sequence num high
    packet_data.push(0x0); // sequence num low

    packet_data.push(0x0); // com version

    let mut control_byte: u8 = 0x00;
    let period = &data.control_mode[0];
    let enabled = &data.control_mode[1];

    match period {
        ControlMode::Teleop => control_byte = control_byte | 0b0000_0000,
        ControlMode::Test => control_byte = control_byte | 0b0000_0001,
        ControlMode::Autonomous => control_byte = control_byte | 0b0000_0010,
        _ => control_byte = control_byte | 0,
    };

    match enabled {
        ControlMode::Enabled => control_byte = control_byte | 0b0000_0100,
        ControlMode::Disabled => control_byte = control_byte | 0b0000_0000,
        _ => control_byte = control_byte | 0b0000_0000,
    };

    packet_data.push(control_byte);

    packet_data.push(0x00); // Request byte

    packet_data.push(data.alliance_station);

    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);
    packet_data.push(0x00);

    return packet_data;
}
