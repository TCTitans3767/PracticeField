use std::{
    fmt::{Display, write},
    net::SocketAddr, sync::Arc,
};

use tokio::{io::AsyncReadExt, net::{TcpStream, UdpSocket}};

use crate::driverstation_comms::driverstation_connection::new_driverstation;

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

pub fn parse_driverstation_tcp(mut data: Vec<u8>) -> anyhow::Result<FromDS> {
    let size_upper = (data.remove(0) as u16) << 8;
    let size_lower = data.remove(0) as u16;

    let id = data.remove(0);

    let mut tag_type = TagType::DSPing(DSPing {});

    match id {
        0x00..=0x07 => {
            // TODO Version stuffs
        }
        0x15 => {
            let team_number_upper = (data.remove(0) as u16) << 8;
            let team_number_lower = data.remove(0) as u16;
            let unknown = data.remove(0);
            let entry_data = EntryData { data: data };
            tag_type = TagType::UsageReport(UsageReport {
                team_num: team_number_upper | team_number_lower,
                unknown: unknown,
                entry_data: entry_data,
            })
        }
        0x16 => {
            let trip_time = data.remove(0);
            let lost_packets = data.remove(0);
            let battery_xx = data.remove(0) as u16;
            let battery_yy = data.remove(0) as u16;
            let robot_status = data.remove(0);
            let can = data.remove(0);
            let signal_db = data.remove(0);
            let bandwidth_high = (data.remove(0) as u16) << 8;
            let bandwidth_low = data.remove(0) as u16;
            tag_type = TagType::LogData(LogData {
                trip_time,
                lost_packets,
                battery: (battery_xx + battery_yy) / 256,
                robot_status,
                can,
                signal_db,
                bandwidth: bandwidth_high | bandwidth_low,
            })
        }
        0x17 => {
            // TODO: Error and event data
        }
        0x18 => {
            let team_number_high = (data.remove(0) as u16) << 8;
            let team_number_low = data.remove(0) as u16;
            tag_type = TagType::TeamNumber(TeamNumber {
                team_number: team_number_high | team_number_low,
            })
        }
        _ => {}
    }

    Ok(FromDS {
        size: size_upper | size_lower,
        tag_id: id,
        tag: tag_type,
    })
}

pub async fn ds_tcp_listener(mut socket: TcpStream, addr: SocketAddr, shared_udp_socket: Arc<UdpSocket>) {
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

        if let Ok(packet) = parse_driverstation_tcp(buf.to_vec()) {
            match packet.tag {
                TagType::TeamNumber(team_number) => {
                    // println!("Team number: {}", team_number.team_number);
                    if team_number_recieved == false {
                        team_number_recieved = true;
                        tokio::spawn(new_driverstation(team_number.team_number, shared_udp_socket.clone()));
                    }
                }
                _ => todo!()
            }
        }
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
