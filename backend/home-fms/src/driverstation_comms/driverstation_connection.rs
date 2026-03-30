use std::{fmt::format, sync::Arc, time::Duration};

use tokio::net::UdpSocket;

use crate::driverstation_comms::{tcp::ds_tcp_listener, udp::{BLUE_1, ControlMode, DSUDPData, create_udp_packet}};

pub struct DriverstationConnection {
    team_number: u16,
    driverstation_udp: DSUDPData,
    ds_control: DSControl
}

pub enum DSControl {
    Uncontrolled,
    FMSFull,
    FMSPartial,
}

pub async fn new_driverstation(team_number: u16, shared_udp_socket: Arc<UdpSocket>) {
    let driverstation_connection = DriverstationConnection{
        team_number,
        driverstation_udp: DSUDPData {
            team_number,
            alliance_station: BLUE_1,
            control_mode: [ControlMode::Teleop, ControlMode::Disabled],
            is_ds_alive: true,
            is_e_stopped: true
        },
        ds_control: DSControl::FMSFull,
    };

    let team_number_string = format!("{team_number}");
    let upper_team_numbers = if team_number_string.chars().count() > 5 {
        &team_number_string[0..4]
    } else {
        &team_number_string[0..3]
    };
    let lower_team_numbers = if team_number_string.chars().count() > 5 {
        &team_number_string[4..6]
    } else {
        &team_number_string[3..5]
    };
    let driverstation_ip = match driverstation_connection.ds_control {
        DSControl::FMSFull => format!("10.{upper_team_numbers}.{lower_team_numbers}.5:1120"),
        DSControl::FMSPartial => format!("10.{upper_team_numbers}.{lower_team_numbers}.5:1121"),
        DSControl::Uncontrolled => format!("0.0.0.0:1120")
    };

    loop {
        shared_udp_socket.send_to(&create_udp_packet(driverstation_connection.driverstation_udp.clone()), &driverstation_ip).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

