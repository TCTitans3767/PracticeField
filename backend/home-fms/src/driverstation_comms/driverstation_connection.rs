use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::net::UdpSocket;

use crate::driverstation_comms::{
    fms::FMS,
    udp::{BLUE_1, ControlMode, create_udp_packet},
};

#[derive(Clone)]
pub struct DriverstationConnection {
    pub team_number: u16,
    pub ds_control: DSControl,
    pub alliance_station: u8,
    pub control_mode: [ControlMode; 2]
}

impl DriverstationConnection {
    pub fn new(team_number: u16) -> Self {
        Self {
            team_number,
            ds_control: DSControl::FMSPartial,
            alliance_station: BLUE_1,
            control_mode: [ControlMode::Teleop, ControlMode::Disabled]
        }
    }
}

#[derive(Clone)]
pub enum DSControl {
    Uncontrolled,
    FMSFull,
    FMSPartial,
}

pub async fn new_driverstation(
    team_number: u16,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>,
) {
    println!(
        "new driverstation control thread created for team {}",
        team_number
    );

    let team_number_string = format!("{}", team_number);
    let upper_team_numbers = if team_number_string.chars().count() > 5 {
        &team_number_string[0..3]
    } else {
        &team_number_string[0..2]
    };
    let lower_team_numbers = if team_number_string.chars().count() > 5 {
        &team_number_string[3..5]
    } else {
        &team_number_string[2..4]
    };

    loop {
        let mut driverstation_ip: String = "".to_string();
        let mut driverstation_connection: DriverstationConnection =
            DriverstationConnection::new(team_number);
        let mut found_driverstation = false;

        {
            let fms_lock = fms.lock().unwrap();
            let driverstation_list = fms_lock.driverstations.clone();

            if let Some(driverstation_connection_position) = driverstation_list
                .iter()
                .position(|driverstation| driverstation.team_number == team_number)
            {
                {
                    driverstation_connection = fms_lock
                        .driverstations
                        .get(driverstation_connection_position)
                        .unwrap()
                        .clone();
                    driverstation_ip = match driverstation_connection.ds_control {
                        DSControl::FMSFull => {
                            format!("10.{upper_team_numbers}.{lower_team_numbers}.5:1120")
                        }
                        DSControl::FMSPartial => {
                            format!("10.{upper_team_numbers}.{lower_team_numbers}.5:1121")
                        }
                        DSControl::Uncontrolled => format!("0.0.0.0:1120"),
                    };
                }
                found_driverstation = true;
            } else {
                println!("could not find drriverstation connection in driverstation vec");
            }
        }
        if found_driverstation {
            _ = shared_udp_socket
                .send_to(
                    &create_udp_packet(&driverstation_connection),
                    &driverstation_ip,
                )
                .await;
            println!("sent udp packet to {}", &driverstation_ip);
        } else {
            println!("could not find driverstation connection for {team_number}; terminating udp connection");
            return;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
