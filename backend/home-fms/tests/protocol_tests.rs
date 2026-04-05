//! # Protocol Tests for home-fms
//!
//! This test suite validates the TCP and UDP protocol implementations for the
//! FIRST Robotics Field Management System (FMS).
//!
//! ## Test Categories
//!
//! - **TCP Parser Tests**: Validates parsing of driverstation TCP packets including
//!   safe byte extraction, tag type parsing, and error handling
//! - **UDP Packet Tests**: Validates creation of UDP control packets sent to robots
//! - **DriverStation Connection Tests**: Validates connection state management
//! - **FMS Tests**: Validates the central FMS state machine and driverstation registry
//! - **Team IP Parsing Tests**: Validates team number to IP address conversion
//! - **Control Mode Tests**: Validates robot control mode enums and states
//! - **Alliance Station Tests**: Validates alliance station constants and assignments
//!
//! ## Protocol Overview
//!
//! ### TCP Protocol (Driverstation → FMS)
//! Packet format: `[size: u16][tag_id: u8][data: ...]`
//! - Tag 0x15: UsageReport
//! - Tag 0x16: LogData (battery, signal, robot status)
//! - Tag 0x18: TeamNumber
//!
//! ### UDP Protocol (FMS → Robot)
//! Packet format: 23 bytes
//! - Bytes 0-1: Sequence number
//! - Byte 2: COM version
//! - Byte 3: Control byte (mode + enable state)
//! - Byte 4: Request byte
//! - Byte 5: Alliance station
//! - Bytes 6-22: Padding (zeros)

use home_fms::driverstation_comms::{
    driverstation_connection::{parse_team_ip_octets, DSControl, DriverstationConnection},
    fms::FMS,
    tcp::{parse_driverstation_tcp, safe_pop, safe_pop_u16, safe_pop_validated, TagType},
    udp::{create_udp_packet, ControlMode, DSUDPData, BLUE_1, BLUE_2, BLUE_3, RED_1, RED_2, RED_3},
};

// ============================================================================
// TCP Parser Tests
// ============================================================================
// Tests for parsing incoming TCP packets from driverstations. These tests
// validate the safe byte extraction helpers and the main packet parser.
// ============================================================================

mod tcp_parser_tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Safe Pop Helper Tests
    // ------------------------------------------------------------------------
    // These tests validate the safe_pop() function which safely extracts
    // single bytes from packet buffers with bounds checking.
    // ------------------------------------------------------------------------

    #[test]
    fn test_safe_pop_single_byte() {
        // Test extracting a single byte (0x42 = 66 decimal)
        let mut data = vec![0x42];
        let result = safe_pop(&mut data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x42);
        assert!(data.is_empty());
    }

    #[test]
    fn test_safe_pop_empty_vector() {
        // Test that popping from empty buffer returns error
        let mut data: Vec<u8> = vec![];
        let result = safe_pop(&mut data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Packet too short"));
    }

    #[test]
    fn test_safe_pop_multiple_bytes() {
        // Test sequential extraction of multiple bytes
        let mut data = vec![0x01, 0x02, 0x03];

        let first = safe_pop(&mut data).unwrap();
        assert_eq!(first, 0x01);
        assert_eq!(data.len(), 2);

        let second = safe_pop(&mut data).unwrap();
        assert_eq!(second, 0x02);
        assert_eq!(data.len(), 1);

        let third = safe_pop(&mut data).unwrap();
        assert_eq!(third, 0x03);
        assert_eq!(data.len(), 0);
    }

    // ------------------------------------------------------------------------
    // Safe Pop U16 Helper Tests
    // ------------------------------------------------------------------------
    // These tests validate safe_pop_u16() which extracts big-endian 16-bit
    // values from packet buffers (used for packet sizes, team numbers, etc.)
    // ------------------------------------------------------------------------

    #[test]
    fn test_safe_pop_u16_big_endian() {
        // Test big-endian extraction: [0x12, 0x34] → 0x1234 (4660 decimal)
        let mut data = vec![0x12, 0x34];
        let result = safe_pop_u16(&mut data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x1234);
        assert!(data.is_empty());
    }

    #[test]
    fn test_safe_pop_u16_zero() {
        // Test extraction of zero value
        let mut data = vec![0x00, 0x00];
        let result = safe_pop_u16(&mut data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x0000);
    }

    #[test]
    fn test_safe_pop_u16_max_value() {
        // Test extraction of maximum u16 value (65535)
        let mut data = vec![0xFF, 0xFF];
        let result = safe_pop_u16(&mut data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0xFFFF);
    }

    #[test]
    fn test_safe_pop_u16_insufficient_bytes() {
        // Test error when only 1 byte available (need 2)
        let mut data = vec![0x12];
        let result = safe_pop_u16(&mut data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Packet too short"));
    }

    #[test]
    fn test_safe_pop_u16_empty_vector() {
        // Test error when buffer is empty
        let mut data: Vec<u8> = vec![];
        let result = safe_pop_u16(&mut data);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Safe Pop Validated Helper Tests
    // ------------------------------------------------------------------------
    // These tests validate safe_pop_validated() which extracts a byte and
    // verifies it's within an acceptable range (used for enum-like values)
    // ------------------------------------------------------------------------

    #[test]
    fn test_safe_pop_validated_in_range() {
        // Test successful validation: value 5 is in range [0, 10]
        let mut data = vec![0x05];
        let result = safe_pop_validated(&mut data, 0, 10, "test_field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x05);
    }

    #[test]
    fn test_safe_pop_validated_at_boundaries() {
        // Test validation at minimum boundary (0)
        let mut data_min = vec![0x00];
        let result_min = safe_pop_validated(&mut data_min, 0, 10, "test_field");
        assert!(result_min.is_ok());
        assert_eq!(result_min.unwrap(), 0x00);

        // Test validation at maximum boundary (10)
        let mut data_max = vec![0x0A];
        let result_max = safe_pop_validated(&mut data_max, 0, 10, "test_field");
        assert!(result_max.is_ok());
        assert_eq!(result_max.unwrap(), 0x0A);
    }

    #[test]
    fn test_safe_pop_validated_out_of_range_low() {
        // Test rejection of value below minimum: 5 < 10
        let mut data = vec![0x05];
        let result = safe_pop_validated(&mut data, 10, 20, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of range"));
    }

    #[test]
    fn test_safe_pop_validated_out_of_range_high() {
        // Test rejection of value above maximum: 21 > 10
        let mut data = vec![0x15];
        let result = safe_pop_validated(&mut data, 0, 10, "test_field");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of range"));
    }

    // ------------------------------------------------------------------------
    // Full Packet Parsing Tests
    // ------------------------------------------------------------------------
    // These tests validate the complete TCP packet parser with real packet
    // structures. Each test constructs a binary packet and verifies correct
    // parsing of the tag type and associated data.
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_team_number_packet() {
        // Test parsing team number packet (tag 0x18)
        // Packet structure: [size: u16][tag: 0x18][team_number: u16]
        let team_number: u16 = 1234;
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0005u16.to_be_bytes()); // size = 5 bytes
        packet.push(0x18); // tag: TeamNumber
        packet.extend_from_slice(&team_number.to_be_bytes()); // team 1234

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_ok());

        let from_ds = result.unwrap();
        assert_eq!(from_ds.size, 0x0005);
        assert_eq!(from_ds.tag_id, 0x18);

        match from_ds.tag {
            TagType::TeamNumber(tn) => assert_eq!(tn.team_number, 1234),
            _ => panic!("Expected TeamNumber tag"),
        }
    }

    #[test]
    fn test_parse_team_number_packet_team_254() {
        // Test parsing team number for team 254 (famous team)
        let team_number: u16 = 254;
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0005u16.to_be_bytes());
        packet.push(0x18);
        packet.extend_from_slice(&team_number.to_be_bytes());

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_ok());

        let from_ds = result.unwrap();
        match from_ds.tag {
            TagType::TeamNumber(tn) => assert_eq!(tn.team_number, 254),
            _ => panic!("Expected TeamNumber tag"),
        }
    }

    #[test]
    fn test_parse_usage_report_packet() {
        // Test parsing usage report packet (tag 0x15)
        // Packet structure: [size][tag: 0x15][team_num][unknown][entry_data...]
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0008u16.to_be_bytes()); // size = 8
        packet.push(0x15); // tag: UsageReport
        packet.extend_from_slice(&1234u16.to_be_bytes()); // team 1234
        packet.push(0x01); // unknown field
        packet.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // entry data

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_ok());

        let from_ds = result.unwrap();
        assert_eq!(from_ds.size, 0x0008);
        assert_eq!(from_ds.tag_id, 0x15);

        match from_ds.tag {
            TagType::UsageReport(ur) => {
                assert_eq!(ur.team_num, 1234);
                assert_eq!(ur.unknown, 0x01);
                assert_eq!(ur.entry_data.data, vec![0xAA, 0xBB, 0xCC]);
            }
            _ => panic!("Expected UsageReport tag"),
        }
    }

    #[test]
    fn test_parse_log_data_packet() {
        // Test parsing log data packet (tag 0x16)
        // Contains: trip_time, lost_packets, battery (2 bytes), robot_status,
        // can, signal_db, bandwidth (2 bytes)
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x000Bu16.to_be_bytes()); // size = 11
        packet.push(0x16); // tag: LogData
        packet.push(0x0A); // trip_time = 10
        packet.push(0x02); // lost_packets = 2
        packet.push(0x08); // battery_xx = 8
        packet.push(0x58); // battery_yy = 88
        packet.push(0x04); // robot_status = 4
        packet.push(0x1E); // can = 30
        packet.push(0x3C); // signal_db = 60
        packet.extend_from_slice(&0x03E8u16.to_be_bytes()); // bandwidth = 1000

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_ok());

        let from_ds = result.unwrap();
        assert_eq!(from_ds.size, 0x000B);
        assert_eq!(from_ds.tag_id, 0x16);

        match from_ds.tag {
            TagType::LogData(ld) => {
                assert_eq!(ld.trip_time, 0x0A);
                assert_eq!(ld.lost_packets, 0x02);
                // Battery voltage = (xx + yy) / 256 = (8 + 88) / 256 = 0.375V
                assert_eq!(ld.battery, (0x08 + 0x58) / 256);
                assert_eq!(ld.robot_status, 0x04);
                assert_eq!(ld.can, 0x1E);
                assert_eq!(ld.signal_db, 0x3C);
                assert_eq!(ld.bandwidth, 0x03E8);
            }
            _ => panic!("Expected LogData tag"),
        }
    }

    #[test]
    fn test_parse_log_data_packet_battery_calculation() {
        // Test battery voltage calculation with different values
        // Battery voltage formula: (xx + yy) / 256
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x000Bu16.to_be_bytes());
        packet.push(0x16);
        packet.push(0x0A);
        packet.push(0x02);
        packet.push(0x07); // battery_xx = 7
        packet.push(0xB1); // battery_yy = 177
        packet.push(0x04);
        packet.push(0x1E);
        packet.push(0x3C);
        packet.extend_from_slice(&0x03E8u16.to_be_bytes());

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_ok());

        match result.unwrap().tag {
            TagType::LogData(ld) => {
                // (7 + 177) / 256 = 0.71875V (truncated to 0)
                assert_eq!(ld.battery, (0x07 + 0xB1) / 256);
            }
            _ => panic!("Expected LogData tag"),
        }
    }

    // ------------------------------------------------------------------------
    // Error Handling Tests
    // ------------------------------------------------------------------------
    // These tests verify that malformed or incomplete packets are properly
    // rejected with appropriate error messages.
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_packet_too_short_team_number() {
        // Test error when team number packet is missing bytes
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0005u16.to_be_bytes());
        packet.push(0x18); // tag: TeamNumber
        packet.push(0x04); // only 1 byte for team number (need 2)

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TeamNumber"));
    }

    #[test]
    fn test_parse_packet_too_short_usage_report() {
        // Test error when usage report packet is missing bytes
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0008u16.to_be_bytes());
        packet.push(0x15); // tag: UsageReport
        packet.push(0x04); // incomplete data

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_packet_too_short_log_data() {
        // Test error when log data packet is missing bytes
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x000Bu16.to_be_bytes());
        packet.push(0x16); // tag: LogData
        packet.push(0x0A);
        packet.push(0x02); // incomplete data

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_packet() {
        // Test error when packet is completely empty
        let packet: Vec<u8> = vec![];
        let result = parse_driverstation_tcp(packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_packet_size_only() {
        // Test error when packet has only size bytes (no tag)
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0001u16.to_be_bytes());

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_tag() {
        // Test handling of unknown/unimplemented tag types
        let mut packet = Vec::new();
        packet.extend_from_slice(&0x0002u16.to_be_bytes());
        packet.push(0x99); // unknown tag
        packet.push(0x00); // padding

        let result = parse_driverstation_tcp(packet);
        assert!(result.is_ok());
        let from_ds = result.unwrap();
        assert_eq!(from_ds.tag_id, 0x99);
    }
}

// ============================================================================
// UDP Packet Tests
// ============================================================================
// Tests for creating UDP control packets sent from FMS to robots.
// Packet structure: 23 bytes total
// - Bytes 0-1: Sequence number (u16)
// - Byte 2: COM version
// - Byte 3: Control byte (period mode + enable state)
// - Byte 4: Request byte
// - Byte 5: Alliance station
// - Bytes 6-22: Padding (zeros)
// ============================================================================

mod udp_packet_tests {
    use super::*;

    #[test]
    fn test_create_udp_packet_basic() {
        // Test basic UDP packet structure
        let ds = DriverstationConnection::new(1234);
        let packet = create_udp_packet(&ds);

        assert_eq!(packet.len(), 23); // Total packet size
        assert_eq!(packet[0], 0x00); // Sequence high
        assert_eq!(packet[1], 0x00); // Sequence low
        assert_eq!(packet[2], 0x00); // COM version
    }

    #[test]
    fn test_create_udp_packet_sequence_numbers() {
        // Test sequence number bytes (always 0 for now)
        let ds = DriverstationConnection::new(1234);
        let packet = create_udp_packet(&ds);

        assert_eq!(packet[0], 0x00);
        assert_eq!(packet[1], 0x00);
    }

    #[test]
    fn test_create_udp_packet_com_version() {
        // Test COM version byte (always 0)
        let ds = DriverstationConnection::new(1234);
        let packet = create_udp_packet(&ds);

        assert_eq!(packet[2], 0x00);
    }

    #[test]
    fn test_create_udp_packet_alliance_station() {
        // Test alliance station byte (default: BLUE_1 = 3)
        let ds = DriverstationConnection::new(1234);
        let packet = create_udp_packet(&ds);

        assert_eq!(packet[5], BLUE_1);
    }

    // ------------------------------------------------------------------------
    // Control Mode Tests
    // ------------------------------------------------------------------------
    // Control byte format: [0 0 0 0 P P E]
    // - PP: Period mode (00=Teleop, 01=Test, 10=Autonomous)
    // - E: Enable state (0=Disabled, 1=Enabled)
    // ------------------------------------------------------------------------

    #[test]
    fn test_create_udp_packet_control_mode_teleop_disabled() {
        // Control byte should be 0b0000_0000 (Teleop + Disabled)
        let mut ds = DriverstationConnection::new(1234);
        ds.control_mode = [ControlMode::Teleop, ControlMode::Disabled];

        let packet = create_udp_packet(&ds);
        let control_byte = packet[3];

        assert_eq!(control_byte & 0b0000_0011, 0b0000_0000); // Period mode
        assert_eq!(control_byte & 0b0000_0100, 0b0000_0000); // Enable state
    }

    #[test]
    fn test_create_udp_packet_control_mode_teleop_enabled() {
        // Control byte should be 0b0000_0100 (Teleop + Enabled)
        let mut ds = DriverstationConnection::new(1234);
        ds.control_mode = [ControlMode::Teleop, ControlMode::Enabled];

        let packet = create_udp_packet(&ds);
        let control_byte = packet[3];

        assert_eq!(control_byte & 0b0000_0011, 0b0000_0000); // Period mode
        assert_eq!(control_byte & 0b0000_0100, 0b0000_0100); // Enable state
    }

    #[test]
    fn test_create_udp_packet_control_mode_autonomous_disabled() {
        // Control byte should be 0b0000_0010 (Autonomous + Disabled)
        let mut ds = DriverstationConnection::new(1234);
        ds.control_mode = [ControlMode::Autonomous, ControlMode::Disabled];

        let packet = create_udp_packet(&ds);
        let control_byte = packet[3];

        assert_eq!(control_byte & 0b0000_0011, 0b0000_0010); // Period mode
        assert_eq!(control_byte & 0b0000_0100, 0b0000_0000); // Enable state
    }

    #[test]
    fn test_create_udp_packet_control_mode_autonomous_enabled() {
        // Control byte should be 0b0000_0110 (Autonomous + Enabled)
        let mut ds = DriverstationConnection::new(1234);
        ds.control_mode = [ControlMode::Autonomous, ControlMode::Enabled];

        let packet = create_udp_packet(&ds);
        let control_byte = packet[3];

        assert_eq!(control_byte & 0b0000_0011, 0b0000_0010); // Period mode
        assert_eq!(control_byte & 0b0000_0100, 0b0000_0100); // Enable state
    }

    #[test]
    fn test_create_udp_packet_control_mode_test_disabled() {
        // Control byte should be 0b0000_0001 (Test + Disabled)
        let mut ds = DriverstationConnection::new(1234);
        ds.control_mode = [ControlMode::Test, ControlMode::Disabled];

        let packet = create_udp_packet(&ds);
        let control_byte = packet[3];

        assert_eq!(control_byte & 0b0000_0011, 0b0000_0001); // Period mode
        assert_eq!(control_byte & 0b0000_0100, 0b0000_0000); // Enable state
    }

    #[test]
    fn test_create_udp_packet_control_mode_test_enabled() {
        // Control byte should be 0b0000_0101 (Test + Enabled)
        let mut ds = DriverstationConnection::new(1234);
        ds.control_mode = [ControlMode::Test, ControlMode::Enabled];

        let packet = create_udp_packet(&ds);
        let control_byte = packet[3];

        assert_eq!(control_byte & 0b0000_0011, 0b0000_0001); // Period mode
        assert_eq!(control_byte & 0b0000_0100, 0b0000_0100); // Enable state
    }

    #[test]
    fn test_create_udp_packet_different_alliance_stations() {
        // Test all 6 alliance station positions
        let mut ds = DriverstationConnection::new(1234);

        ds.alliance_station = RED_1;
        let packet_red1 = create_udp_packet(&ds);
        assert_eq!(packet_red1[5], RED_1);

        ds.alliance_station = RED_2;
        let packet_red2 = create_udp_packet(&ds);
        assert_eq!(packet_red2[5], RED_2);

        ds.alliance_station = RED_3;
        let packet_red3 = create_udp_packet(&ds);
        assert_eq!(packet_red3[5], RED_3);

        ds.alliance_station = BLUE_1;
        let packet_blue1 = create_udp_packet(&ds);
        assert_eq!(packet_blue1[5], BLUE_1);

        ds.alliance_station = BLUE_2;
        let packet_blue2 = create_udp_packet(&ds);
        assert_eq!(packet_blue2[5], BLUE_2);

        ds.alliance_station = BLUE_3;
        let packet_blue3 = create_udp_packet(&ds);
        assert_eq!(packet_blue3[5], BLUE_3);
    }

    #[test]
    fn test_create_udp_packet_padding_bytes() {
        // Test that all padding bytes (6-22) are zero
        let ds = DriverstationConnection::new(1234);
        let packet = create_udp_packet(&ds);

        for i in 6..23 {
            assert_eq!(packet[i], 0x00);
        }
    }

    #[test]
    fn test_create_udp_packet_different_team_numbers() {
        // Test packet creation for various team numbers
        let teams = vec![1, 254, 1234, 9999, 12345];

        for team in teams {
            let ds = DriverstationConnection::new(team);
            let packet = create_udp_packet(&ds);
            assert_eq!(packet.len(), 23);
        }
    }

    #[test]
    fn test_dsudpdata_new() {
        // Test DSUDPData constructor
        let data = DSUDPData::new(1234);

        assert_eq!(data.team_number, 1234);
        assert_eq!(data.alliance_station, BLUE_1);
        assert_eq!(data.control_mode.len(), 2);
    }

    #[test]
    fn test_dsudpdata_default_control_modes() {
        // Test default control modes (Teleop + Disabled)
        let data = DSUDPData::new(1234);

        match data.control_mode[0] {
            ControlMode::Teleop => (),
            _ => panic!("Expected Teleop as first control mode"),
        }

        match data.control_mode[1] {
            ControlMode::Disabled => (),
            _ => panic!("Expected Disabled as second control mode"),
        }
    }
}

// ============================================================================
// DriverStation Connection Tests
// ============================================================================
// Tests for the DriverstationConnection struct which represents a connected
// driverstation and its current state (team number, control mode, alliance).
// ============================================================================

mod driverstation_connection_tests {
    use super::*;

    #[test]
    fn test_driverstation_connection_new() {
        // Test default connection creation
        let ds = DriverstationConnection::new(1234);

        assert_eq!(ds.team_number, 1234);
        assert!(matches!(ds.ds_control, DSControl::FMSPartial));
        assert_eq!(ds.alliance_station, BLUE_1);
        assert_eq!(ds.control_mode.len(), 2);
    }

    #[test]
    fn test_driverstation_connection_default_control_modes() {
        // Test default control modes (Teleop + Disabled)
        let ds = DriverstationConnection::new(1234);

        match ds.control_mode[0] {
            ControlMode::Teleop => (),
            _ => panic!("Expected Teleop as default period mode"),
        }

        match ds.control_mode[1] {
            ControlMode::Disabled => (),
            _ => panic!("Expected Disabled as default enabled state"),
        }
    }

    #[test]
    fn test_driverstation_connection_different_teams() {
        // Test connection creation for various team numbers
        let teams = vec![1, 254, 1234, 9999, 12345];

        for team in teams {
            let ds = DriverstationConnection::new(team);
            assert_eq!(ds.team_number, team);
        }
    }

    #[test]
    fn test_driverstation_connection_clone() {
        // Test cloning a connection
        let ds1 = DriverstationConnection::new(1234);
        let ds2 = ds1.clone();

        assert_eq!(ds1.team_number, ds2.team_number);
        assert_eq!(ds1.alliance_station, ds2.alliance_station);
    }

    #[test]
    fn test_driverstation_connection_control_mode_modification() {
        // Test modifying control modes
        let mut ds = DriverstationConnection::new(1234);

        ds.control_mode = [ControlMode::Autonomous, ControlMode::Enabled];
        assert!(matches!(ds.control_mode[0], ControlMode::Autonomous));
        assert!(matches!(ds.control_mode[1], ControlMode::Enabled));

        ds.control_mode = [ControlMode::Test, ControlMode::Disabled];
        assert!(matches!(ds.control_mode[0], ControlMode::Test));
        assert!(matches!(ds.control_mode[1], ControlMode::Disabled));
    }

    #[test]
    fn test_driverstation_connection_alliance_station_modification() {
        // Test changing alliance station assignment
        let mut ds = DriverstationConnection::new(1234);

        ds.alliance_station = RED_1;
        assert_eq!(ds.alliance_station, RED_1);

        ds.alliance_station = BLUE_3;
        assert_eq!(ds.alliance_station, BLUE_3);
    }

    #[test]
    fn test_driverstation_connection_ds_control_modification() {
        // Test changing DS control state
        let mut ds = DriverstationConnection::new(1234);

        ds.ds_control = DSControl::FMSFull;
        assert!(matches!(ds.ds_control, DSControl::FMSFull));

        ds.ds_control = DSControl::Uncontrolled;
        assert!(matches!(ds.ds_control, DSControl::Uncontrolled));
    }
}

// ============================================================================
// FMS Tests
// ============================================================================
// Tests for the Field Management System (FMS) state machine which manages
// driverstation connections, match configuration, and team allowances.
// ============================================================================

mod fms_tests {
    use super::*;

    #[test]
    fn test_fms_default() {
        // Test default FMS state (empty)
        let fms = FMS::default();

        assert!(fms.driverstations.is_empty());
        assert!(fms.allowed_driverstations.is_empty());
    }

    #[test]
    fn test_fms_add_ds_first_team() {
        // Test adding first driverstation
        let mut fms = FMS::default();
        fms.add_ds(1234);

        assert_eq!(fms.driverstations.len(), 1);
        assert_eq!(fms.driverstations[0].team_number, 1234);
    }

    #[test]
    fn test_fms_add_ds_multiple_teams() {
        // Test adding multiple driverstations
        let mut fms = FMS::default();
        fms.add_ds(1234);
        fms.add_ds(254);
        fms.add_ds(1);

        assert_eq!(fms.driverstations.len(), 3);
        assert!(fms.driverstations.iter().any(|ds| ds.team_number == 1234));
        assert!(fms.driverstations.iter().any(|ds| ds.team_number == 254));
        assert!(fms.driverstations.iter().any(|ds| ds.team_number == 1));
    }

    #[test]
    fn test_fms_add_ds_duplicate_prevention() {
        // Test that duplicate team additions are prevented
        let mut fms = FMS::default();
        fms.add_ds(1234);
        fms.add_ds(1234);
        fms.add_ds(1234);

        assert_eq!(fms.driverstations.len(), 1);
        assert_eq!(fms.driverstations[0].team_number, 1234);
    }

    #[test]
    fn test_fms_remove_ds_existing() {
        // Test removing an existing driverstation
        let mut fms = FMS::default();
        fms.add_ds(1234);
        fms.add_ds(254);

        assert_eq!(fms.driverstations.len(), 2);

        fms.remove_ds(1234);

        assert_eq!(fms.driverstations.len(), 1);
        assert_eq!(fms.driverstations[0].team_number, 254);
    }

    #[test]
    fn test_fms_remove_ds_nonexistent() {
        // Test removing a non-existent driverstation (should be no-op)
        let mut fms = FMS::default();
        fms.add_ds(1234);

        fms.remove_ds(9999);

        assert_eq!(fms.driverstations.len(), 1);
    }

    #[test]
    fn test_fms_remove_ds_empty_fms() {
        // Test removing from empty FMS (should be no-op)
        let mut fms = FMS::default();

        fms.remove_ds(1234);

        assert_eq!(fms.driverstations.len(), 0);
    }

    #[test]
    fn test_fms_add_to_match() {
        // Test adding a team to the match (full FMS control)
        let mut fms = FMS::default();
        fms.add_to_match(1234, BLUE_1);

        assert_eq!(fms.driverstations.len(), 1);
        assert_eq!(fms.allowed_driverstations.len(), 1);
        assert_eq!(fms.allowed_driverstations[0], 1234);

        let ds = fms.get_ds(1234).unwrap();
        assert_eq!(ds.alliance_station, BLUE_1);
        assert!(matches!(ds.ds_control, DSControl::FMSFull));
    }

    #[test]
    fn test_fms_add_to_match_multiple_teams() {
        // Test adding multiple teams to a match
        let mut fms = FMS::default();
        fms.add_to_match(1234, BLUE_1);
        fms.add_to_match(254, RED_1);
        fms.add_to_match(1, BLUE_2);

        assert_eq!(fms.driverstations.len(), 3);
        assert_eq!(fms.allowed_driverstations.len(), 3);

        let ds1 = fms.get_ds(1234).unwrap();
        assert_eq!(ds1.alliance_station, BLUE_1);

        let ds2 = fms.get_ds(254).unwrap();
        assert_eq!(ds2.alliance_station, RED_1);

        let ds3 = fms.get_ds(1).unwrap();
        assert_eq!(ds3.alliance_station, BLUE_2);
    }

    #[test]
    fn test_fms_get_ds_existing() {
        // Test retrieving an existing driverstation
        let mut fms = FMS::default();
        fms.add_ds(1234);

        let ds = fms.get_ds(1234);
        assert!(ds.is_some());
        assert_eq!(ds.unwrap().team_number, 1234);
    }

    #[test]
    fn test_fms_get_ds_nonexistent() {
        // Test retrieving a non-existent driverstation
        let mut fms = FMS::default();
        fms.add_ds(1234);

        let ds = fms.get_ds(9999);
        assert!(ds.is_none());
    }

    #[test]
    fn test_fms_get_ds_empty() {
        // Test retrieving from empty FMS
        let mut fms = FMS::default();

        let ds = fms.get_ds(1234);
        assert!(ds.is_none());
    }

    #[test]
    fn test_fms_get_ds_modification() {
        // Test modifying driverstation via get_ds reference
        let mut fms = FMS::default();
        fms.add_ds(1234);

        if let Some(ds) = fms.get_ds(1234) {
            ds.alliance_station = RED_1;
            ds.ds_control = DSControl::FMSFull;
        }

        let ds = fms.get_ds(1234).unwrap();
        assert_eq!(ds.alliance_station, RED_1);
        assert!(matches!(ds.ds_control, DSControl::FMSFull));
    }

    #[test]
    fn test_fms_add_ds_with_allowed_list_empty() {
        // Test adding DS when allowed list is empty (open mode)
        let mut fms = FMS::default();
        fms.allowed_driverstations = vec![];

        fms.add_ds(1234);

        assert_eq!(fms.driverstations.len(), 1);
    }

    #[test]
    fn test_fms_add_ds_with_allowed_list_contains_team() {
        // Test adding DS when team is in allowed list (restricted mode)
        let mut fms = FMS::default();
        fms.allowed_driverstations = vec![1234, 254];

        fms.add_ds(1234);
        fms.add_ds(254);

        assert_eq!(fms.driverstations.len(), 2);
    }

    #[test]
    fn test_fms_add_ds_with_allowed_list_does_not_contain_team() {
        // Test adding DS when team is NOT in allowed list (rejected)
        let mut fms = FMS::default();
        fms.allowed_driverstations = vec![1234];

        fms.add_ds(9999);

        assert_eq!(fms.driverstations.len(), 0);
    }
}

// ============================================================================
// Team IP Parsing Tests
// ============================================================================
// Tests for converting team numbers to IP address octets.
// FIRST robotics uses team-based IP addressing:
// - Team ZXXYY → IP 10.ZXX.YY.5
// - Small teams (00001-09999): Z=0, so 10.XX.YY.5
// - Large teams (10000-65535): Z≠0, so 10.ZXX.YY.5
// ============================================================================

mod team_ip_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_team_ip_octets_small_team_numbers() {
        // Test small team numbers (first digit is 0)
        // Team 1 → padded "00001" → split as "00" and "01"
        let (upper, lower) = parse_team_ip_octets(1).unwrap();
        assert_eq!(upper, "00");
        assert_eq!(lower, "01");

        // Team 254 → padded "00254" → split as "02" and "54"
        let (upper, lower) = parse_team_ip_octets(254).unwrap();
        assert_eq!(upper, "02");
        assert_eq!(lower, "54");

        // Team 1234 → padded "01234" → split as "12" and "34"
        let (upper, lower) = parse_team_ip_octets(1234).unwrap();
        assert_eq!(upper, "12");
        assert_eq!(lower, "34");
    }

    #[test]
    fn test_parse_team_ip_octets_large_team_numbers() {
        // Test large team numbers (first digit is not 0)
        // Team 12345 → "12345" → split as "123" and "45"
        let (upper, lower) = parse_team_ip_octets(12345).unwrap();
        assert_eq!(upper, "123");
        assert_eq!(lower, "45");

        // Team 10000 → "10000" → split as "100" and "00"
        let (upper, lower) = parse_team_ip_octets(10000).unwrap();
        assert_eq!(upper, "100");
        assert_eq!(lower, "00");

        // Team 9999 → "09999" → split as "99" and "99"
        let (upper, lower) = parse_team_ip_octets(9999).unwrap();
        assert_eq!(upper, "99");
        assert_eq!(lower, "99");
    }

    #[test]
    fn test_parse_team_ip_octets_boundary_cases() {
        // Test boundary cases
        // Team 100 → "00100" → split as "01" and "00"
        let (upper, lower) = parse_team_ip_octets(100).unwrap();
        assert_eq!(upper, "01");
        assert_eq!(lower, "00");

        // Team 1000 → "01000" → split as "10" and "00"
        let (upper, lower) = parse_team_ip_octets(1000).unwrap();
        assert_eq!(upper, "10");
        assert_eq!(lower, "00");

        // Team 0 → "00000" → split as "00" and "00"
        let (upper, lower) = parse_team_ip_octets(0).unwrap();
        assert_eq!(upper, "00");
        assert_eq!(lower, "00");
    }

    #[test]
    fn test_parse_team_ip_octets_max_u16() {
        // Test maximum u16 value (65535)
        let (upper, lower) = parse_team_ip_octets(u16::MAX).unwrap();
        assert_eq!(upper, "655");
        assert_eq!(lower, "35");
    }

    #[test]
    fn test_parse_team_ip_octets_specific_teams() {
        // Test specific well-known teams
        let test_cases = vec![
            (1, "00", "01"),      // Team 1
            (254, "02", "54"),    // Team 254 (The Cheesy Poofs)
            (1000, "10", "00"),   // Team 1000
            (1234, "12", "34"),   // Team 1234
            (9999, "99", "99"),   // Team 9999
            (12345, "123", "45"), // Team 12345
        ];

        for (team, expected_upper, expected_lower) in test_cases {
            let (upper, lower) = parse_team_ip_octets(team).unwrap();
            assert_eq!(upper, expected_upper, "Failed for team {}", team);
            assert_eq!(lower, expected_lower, "Failed for team {}", team);
        }
    }
}

// ============================================================================
// Control Mode Tests
// ============================================================================
// Tests for the ControlMode enum which represents robot operational states.
// Modes: EStop, AStop, Enabled, Disabled, Teleop, Autonomous, Test
// ============================================================================

mod control_mode_tests {
    use super::*;

    #[test]
    fn test_control_mode_clone() {
        // Test cloning control mode
        let mode1 = ControlMode::Teleop;
        let mode2 = mode1.clone();

        assert!(matches!(mode2, ControlMode::Teleop));
    }

    #[test]
    fn test_control_mode_variants_exist() {
        // Test that all control mode variants exist
        let modes = vec![
            ControlMode::EStop,
            ControlMode::AStop,
            ControlMode::Enabled,
            ControlMode::Disabled,
            ControlMode::Teleop,
            ControlMode::Autonomous,
            ControlMode::Test,
        ];

        assert_eq!(modes.len(), 7);
    }

    #[test]
    fn test_control_mode_in_array() {
        // Test control modes in array (used in DriverstationConnection)
        let modes: [ControlMode; 2] = [ControlMode::Teleop, ControlMode::Enabled];

        assert!(matches!(modes[0], ControlMode::Teleop));
        assert!(matches!(modes[1], ControlMode::Enabled));
    }
}

// ============================================================================
// Alliance Station Tests
// ============================================================================
// Tests for alliance station constants.
// Red alliance: 0, 1, 2 (RED_1, RED_2, RED_3)
// Blue alliance: 3, 4, 5 (BLUE_1, BLUE_2, BLUE_3)
// ============================================================================

mod alliance_station_tests {
    use super::*;

    #[test]
    fn test_alliance_station_constants() {
        // Verify alliance station constant values
        assert_eq!(RED_1, 0);
        assert_eq!(RED_2, 1);
        assert_eq!(RED_3, 2);
        assert_eq!(BLUE_1, 3);
        assert_eq!(BLUE_2, 4);
        assert_eq!(BLUE_3, 5);
    }

    #[test]
    fn test_alliance_station_range() {
        // Test that stations are sequential 0-5
        let stations = vec![RED_1, RED_2, RED_3, BLUE_1, BLUE_2, BLUE_3];

        for (i, &station) in stations.iter().enumerate() {
            assert_eq!(station, i as u8);
        }
    }
}
