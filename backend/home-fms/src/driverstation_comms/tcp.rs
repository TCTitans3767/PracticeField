use std::{
    fmt::{write, Display},
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use tokio::{
    io::AsyncReadExt,
    net::{TcpStream, UdpSocket},
};

use crate::driverstation_comms::{
    driverstation_connection::{new_driverstation, DriverstationConnection},
    fms::FMS,
};

pub struct ToDS {}

pub struct FromDS {
    pub size: u16,
    pub tag_id: u8,
    pub tag: TagType,
}

pub enum TagType {
    Version(VersionTag),
    UsageReport(UsageReport),
    LogData(LogData),
    ErrorEventData(ErrorEventData),
    TeamNumber(TeamNumber),
    ChallengeResponse(ChallengeResponse),
    DSPing(DSPing),
}

pub struct VersionTag {
    status: String,
    version: String,
}

pub struct UsageReport {
    team_num: u16,
    unknown: u8,
    entry_data: EntryData,
}

pub struct LogData {
    trip_time: u8,
    lost_packets: u8,
    battery: u16, // recieved as xxyy, (xx + yy)/256 = voltage
    robot_status: u8,
    can: u8,       // value is halved
    signal_db: u8, // value is halved
    bandwidth: u16,
}

pub struct ErrorEventData {
    message_count: u32,
    timestamp: u64,
    unknown: u64, // should always be 0x86 0x48 0xb0 0x00 0x00 0x00 0x00 0x00
    log_message: String,
}

pub struct TeamNumber {
    team_number: u16,
}

pub struct ChallengeResponse {}

pub struct DSPing {}

pub struct EntryData {
    data: Vec<u8>,
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
fn safe_pop(data: &mut Vec<u8>) -> anyhow::Result<u8> {
    if data.is_empty() {
        anyhow::bail!("Packet too short: expected at least 1 more byte, got 0");
    }
    Ok(data.remove(0))
}

/// Safely extract a big-endian u16 (two consecutive bytes)
fn safe_pop_u16(data: &mut Vec<u8>) -> anyhow::Result<u16> {
    let upper = safe_pop(data)? as u16;
    let lower = safe_pop(data)? as u16;
    Ok((upper << 8) | lower)
}

/// Safely extract and validate a u8 field is within expected range
fn safe_pop_validated(
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
    fms: Arc<Mutex<FMS>>,
) {
    let mut team_number_recieved = false;
    loop {
        let mut buf = [0; 1024];

        let n = match socket.read(&mut buf).await {
            Ok(n) if n == 0 => return,
            Ok(n) => n,
            Err(e) => {
                eprintln!("failed to read: {e}");
                return;
            }
        };

        println!("recieved {} bytes from {}", n, addr);

        match parse_driverstation_tcp(buf[..n].to_vec()) {
            Ok(packet) => match packet.tag {
                TagType::TeamNumber(team_number) => {
                    if !team_number_recieved {
                        team_number_recieved = true;
                        match fms.lock() {
                            Ok(mut fms_mut) => {
                                fms_mut.add_ds(team_number.team_number);
                                tokio::spawn(new_driverstation(
                                    team_number.team_number,
                                    shared_udp_socket.clone(),
                                    fms.clone(),
                                ));
                            }
                            Err(e) => {
                                eprintln!("Failed to acquire FMS lock: {e}");
                                return;
                            }
                        }
                    }
                }
                TagType::Version(_) => {
                    eprintln!("Version tag parsing not yet implemented");
                }
                TagType::UsageReport(_) => {
                    eprintln!("Usage report processing not yet implemented");
                }
                TagType::LogData(_) => {
                    eprintln!("Log data processing not yet implemented");
                }
                TagType::ErrorEventData(_) => {
                    eprintln!("Error event data processing not yet implemented");
                }
                TagType::ChallengeResponse(_) => {
                    eprintln!("Challenge response processing not yet implemented");
                }
                TagType::DSPing(_) => {
                    eprintln!("DS ping processing not yet implemented");
                }
            },
            Err(e) => {
                eprintln!("Malformed Packet: \n     {e}");
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
