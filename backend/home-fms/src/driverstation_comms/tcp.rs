use std::{
    fmt::Display,
    net::SocketAddr,
    sync::Arc,
};

use anyhow::Context;
use tokio::{
    io::AsyncReadExt,
    net::{TcpStream, UdpSocket},
};
use tracing::{debug, warn};

use crate::driverstation_comms::{
    driverstation_connection::new_driverstation,
    fms_commands::get_fms_queue,
};

pub struct ToDS {}

#[derive(Debug)]
pub struct FromDS {
    pub size: u16,
    pub tag_id: u8,
    pub tag: TagType,
}

#[derive(Debug)]
pub enum TagType {
    Version(VersionTag),
    UsageReport(UsageReport),
    LogData(LogData),
    ErrorEventData(ErrorEventData),
    TeamNumber(TeamNumber),
    ChallengeResponse(ChallengeResponse),
    DSPing(DSPing),
}

#[derive(Debug)]
pub struct VersionTag {
    pub status: String,
    pub version: String,
}

#[derive(Debug)]
pub struct UsageReport {
    pub team_num: u16,
    pub unknown: u8,
    pub entry_data: EntryData,
}

#[derive(Debug)]
pub struct LogData {
    pub trip_time: u8,
    pub lost_packets: u8,
    pub battery: u16, // recieved as xxyy, (xx + yy)/256 = voltage
    pub robot_status: u8,
    pub can: u8,       // value is halved
    pub signal_db: u8, // value is halved
    pub bandwidth: u16,
}

#[derive(Debug)]
pub struct ErrorEventData {
    pub message_count: u32,
    pub timestamp: u64,
    pub unknown: u64, // should always be 0x86 0x48 0xb0 0x00 0x00 0x00 0x00 0x00
    pub log_message: String,
}

#[derive(Debug)]
pub struct TeamNumber {
    pub team_number: u16,
}

#[derive(Debug)]
pub struct ChallengeResponse {}

#[derive(Debug)]
pub struct DSPing {}

#[derive(Debug)]
pub struct EntryData {
    pub data: Vec<u8>,
}

pub const BROWNOUT_MASK: u8 = 0b1000_0000;
pub const WATCHDOG_MASK: u8 = 0b0100_0000;
pub const DS_TELEOP_MASK: u8 = 0b0010_0000;
pub const DS_AUTO_MASK: u8 = 0b0001_0000;
pub const DS_DISABLE_MASK: u8 = 0b0000_1000;
pub const ROBOT_TELEOP_MASK: u8 = 0b0000_0100;
pub const ROBOT_AUTO_MASK: u8 = 0b0000_0010;
pub const ROBOT_DISABLE_MASK: u8 = 0b0000_0001;

/// Safely remove and return a single byte from the packet, with context
pub fn safe_pop(data: &mut Vec<u8>) -> anyhow::Result<u8> {
    if data.is_empty() {
        anyhow::bail!("Packet too short: expected at least 1 more byte, got 0");
    }
    Ok(data.remove(0))
}

/// Safely extract a big-endian u16 (two consecutive bytes)
pub fn safe_pop_u16(data: &mut Vec<u8>) -> anyhow::Result<u16> {
    let upper = safe_pop(data)? as u16;
    let lower = safe_pop(data)? as u16;
    Ok((upper << 8) | lower)
}

/// Safely extract and validate a u8 field is within expected range
pub fn safe_pop_validated(
    data: &mut Vec<u8>,
    min: u8,
    max: u8,
    field_name: &str,
) -> anyhow::Result<u8> {
    let value = safe_pop(data)?;
    if value < min || value > max {
        anyhow::bail!(
            "Field {} value {} out of range [{}, {}]",
            field_name,
            value,
            min,
            max
        );
    }
    Ok(value)
}

pub fn parse_driverstation_tcp(mut data: Vec<u8>) -> anyhow::Result<FromDS> {
    let size =
        safe_pop_u16(&mut data).context("TCP Parser: Could not parse size; expected two bytes")?;

    let tag_id =
        safe_pop(&mut data).context("TCP Parser: Could not parse tag_id; expected one byte")?;

    let mut tag_type = TagType::DSPing(DSPing {});

    match tag_id {
        0x00..=0x07 => {
            // TODO Version stuffs
        }
        0x15 => {
            // UsageReport parsing

            let team_num = safe_pop_u16(&mut data)
                .context("UsageReport: failed to parse team_num, 2 bytes required")?;

            let unknown = safe_pop(&mut data).context(
                "UsageReport: failed to parse unknown value, should be one byte in theory",
            )?;

            tag_type = TagType::UsageReport(UsageReport {
                team_num,
                unknown,
                entry_data: EntryData { data: data },
            })
        }
        0x16 => {
            let trip_time = safe_pop(&mut data)
                .context("LogData: Could not parse trip_time, expected one byte")?;

            let lost_packets = safe_pop(&mut data)
                .context("LogData: Could not parse lost_packets, expected one byte")?;

            let battery_xx = safe_pop(&mut data)
                .context("LogData: Could not parse battery_xx, expected one byte")?
                as u16;

            let battery_yy = safe_pop(&mut data)
                .context("LogData: Could not parse battery_yy, expected one byte")?
                as u16;

            let robot_status = safe_pop(&mut data)
                .context("LogData: Could not parse robot_status, expected one byte")?;

            let can =
                safe_pop(&mut data).context("LogData: Could not parse can, expected one byte")?;

            let signal_db = safe_pop(&mut data)
                .context("LogData: Could not parse signal_db, expected one byte")?;

            let bandwidth = safe_pop_u16(&mut data)
                .context("LogData: could not parse bandwith, expected two bytes")?;

            let battery = (battery_xx + battery_yy) / 256;

            tag_type = TagType::LogData(LogData {
                trip_time,
                lost_packets,
                battery,
                robot_status,
                can,
                signal_db,
                bandwidth,
            })
        }
        0x17 => {
            // TODO: Error and event data
        }
        0x18 => {
            let team_number = safe_pop_u16(&mut data)
                .context("TeamNumber: Could not parse team_number; expected two bytes")?;

            tag_type = TagType::TeamNumber(TeamNumber { team_number })
        }
        _ => {}
    }

    Ok(FromDS {
        size,
        tag_id,
        tag: tag_type,
    })
}

pub async fn ds_tcp_listener(
    mut socket: TcpStream,
    addr: SocketAddr,
    shared_udp_socket: Arc<UdpSocket>,
) {
    let mut team_number_recieved = false;
    let fms_queue = get_fms_queue();
    
    loop {
        let mut buf = [0; 1024];

        let n = match socket.read(&mut buf).await {
            Ok(n) if n == 0 => return,
            Ok(n) => n,
            Err(e) => {
                warn!("failed to read: {e}");
                return;
            }
        };

        debug!("recieved {} bytes from {}", n, addr);

        match parse_driverstation_tcp(buf[..n].to_vec()) {
            Ok(packet) => match packet.tag {
                TagType::TeamNumber(team_number) => {
                    if !team_number_recieved {
                        team_number_recieved = true;
                        
                        // Add driverstation via command queue (no mutex needed!)
                        fms_queue.add_driver_station(team_number.team_number).await;
                        
                        // Spawn UDP controller for this team
                        tokio::spawn(new_driverstation(
                            team_number.team_number,
                            shared_udp_socket.clone(),
                        ));
                    }
                }
                TagType::Version(_) => {
                    warn!("Version tag parsing not yet implemented");
                }
                TagType::UsageReport(_) => {
                    warn!("Usage report processing not yet implemented");
                }
                TagType::LogData(_) => {
                    warn!("Log data processing not yet implemented");
                }
                TagType::ErrorEventData(_) => {
                    warn!("Error event data processing not yet implemented");
                }
                TagType::ChallengeResponse(_) => {
                    warn!("Challenge response processing not yet implemented");
                }
                TagType::DSPing(_) => {
                    warn!("DS ping processing not yet implemented");
                }
            },
            Err(e) => {
                warn!("Malformed Packet: \n     {e}");
            }
        };
    }
}

impl Display for TagType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Version(_) => write!(f, "Version"),
            Self::UsageReport(_) => write!(f, "Usage Report"),
            Self::LogData(_) => write!(f, "Log Data"),
            Self::ErrorEventData(_) => write!(f, "Error and Event Data"),
            Self::TeamNumber(team_number) => write!(f, "Team Number {}", team_number.team_number),
            Self::DSPing(_) => write!(f, "Driverstation Ping"),
            Self::ChallengeResponse(_) => write!(f, "Challenge Response"),
        }
    }
}
