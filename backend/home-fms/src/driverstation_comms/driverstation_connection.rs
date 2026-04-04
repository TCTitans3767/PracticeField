use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use tokio::net::UdpSocket;
use tracing::{debug, event, warn, Level};

use crate::driverstation_comms::{
    fms_commands::get_fms_queue,
    udp::{create_udp_packet, ControlMode, BLUE_1},
};

#[derive(Clone, Debug)]
pub struct DriverstationConnection {
    pub team_number: u16,
    pub ds_control: DSControl,
    pub alliance_station: u8,
    /// Robot control mode: [period_mode, enabled_state]
    /// Index 0: ControlMode::Teleop, Test, or Autonomous
    /// Index 1: ControlMode::Enabled or Disabled
    /// SAFETY: Must always be exactly 2 elements - enforced by type system
    pub control_mode: [ControlMode; 2],
}

impl DriverstationConnection {
    pub fn new(team_number: u16) -> Self {
        Self {
            team_number,
            ds_control: DSControl::FMSPartial,
            alliance_station: BLUE_1,
            control_mode: [ControlMode::Teleop, ControlMode::Disabled],
        }
    }
}

#[derive(Clone, Debug)]
pub enum DSControl {
    Uncontrolled,
    FMSFull,
    FMSPartial,
}

/// Parse FIRST team number into IP octets
///
/// FIRST robotics uses team number to derive team IP:
/// Team ZXXYY   -> IP 10.ZXX.YY.5
///
/// Examples:
/// - Team 0001  -> 10.00.01.5
/// - Team 0254  -> 10.02.54.5
/// - Team 1234  -> 10.12.34.5
/// - Team 12345 -> 10.123.45.5
pub fn parse_team_ip_octets(team_number: u16) -> anyhow::Result<(String, String)> {
    // Pad team number to 5 digits
    let padded = format!("{:05}", team_number);

    if padded.len() != 5 {
        anyhow::bail!(
            "Team number {} produced invalid format: {} (len={})",
            team_number,
            padded,
            padded.len()
        );
    }

    let upper: String;
    let lower: String;

    let first_number = padded.chars().nth(0).unwrap();

    if first_number.to_string() == "0" {
        // Split ZXXYY into XX and YY
        upper = padded[1..3].to_string();
        lower = padded[3..5].to_string();
    } else {
        // Split ZXXYY into ZXX and YY
        upper = padded[0..3].to_string();
        lower = padded[3..5].to_string();
    }

    debug!(team_number, upper, lower, "Team number parsed to IP octets");

    Ok((upper, lower))
}

pub async fn new_driverstation(
    team_number: u16,
    shared_udp_socket: Arc<UdpSocket>,
) -> anyhow::Result<()> {
    debug!(
        "new driverstation control thread created for team {}",
        team_number
    );

    let (upper_team_numbers, lower_team_numbers) =
        parse_team_ip_octets(team_number).context("Failed to parse team ip octets from team number")?;

    let fms_queue = get_fms_queue();

    loop {
        // Query FMS state via command queue (no mutex!)
        let ds_connection = fms_queue.get_driver_station(team_number).await;

        if let Some(driverstation_connection) = ds_connection {
            let driverstation_ip = match driverstation_connection.ds_control {
                DSControl::FMSFull => {
                    format!("10.{upper_team_numbers}.{lower_team_numbers}.5:1120")
                }
                DSControl::FMSPartial => {
                    format!("10.{upper_team_numbers}.{lower_team_numbers}.5:1121")
                }
                DSControl::Uncontrolled => format!("0.0.0.0:1120"),
            };

            match shared_udp_socket
                .send_to(
                    &create_udp_packet(&driverstation_connection),
                    &driverstation_ip,
                )
                .await
            {
                Ok(_) => debug!("sent udp packet to {}", &driverstation_ip),
                Err(e) => {
                    event!(
                        Level::ERROR,
                        "Failed to send UDP packet to {}: {e}",
                        driverstation_ip
                    );
                }
            }
        } else {
            warn!("could not find driverstation connection for {team_number}; terminating udp connection");
            anyhow::bail!(
                "udp connection terminated for {team_number}; driverstation connection not found"
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
