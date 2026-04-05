# Phase 1: Codebase Cleanup & Architecture

**Objective:** Eliminate panics, improve error handling, complete TCP protocol implementation, establish logging infrastructure.

**Duration:** ~2.5 hours  
**Status:** Ready to start

---

## Overview

Phase 1 focuses on stabilizing the codebase so it can handle edge cases gracefully. A single malformed packet currently crashes the entire connection thread. This phase implements defensive programming practices and proper error handling throughout the stack.

### Goals
- ✅ Zero panics on malformed input
- ✅ Graceful degradation of errors
- ✅ Complete TCP protocol implementation
- ✅ Structured logging for debugging
- ✅ Connection timeout mechanisms

---

## P1.1: Error Handling & Robustness

### P1.1.1: TCP Packet Parsing - Safe Vector Access

#### Problem
The `parse_driverstation_tcp()` function uses `.remove(0)` without bounds checking, causing panics on malformed packets.

**Current buggy code (tcp.rs:77-129):**
```rust
pub fn parse_driverstation_tcp(mut data: Vec<u8>) -> anyhow::Result<FromDS> {
    let size_upper = (data.remove(0) as u16) << 8;  // 🔴 PANIC if empty!
    let size_lower = data.remove(0) as u16;          // 🔴 PANIC if < 2 bytes!
    let id = data.remove(0);                         // 🔴 PANIC if < 3 bytes!

    match id {
        0x15 => {
            let team_number_upper = (data.remove(0) as u16) << 8;  // 🔴 Unsafe!
            let team_number_lower = data.remove(0) as u16;          // 🔴 Unsafe!
            // ... more unsafe removes
        }
        0x16 => {
            // 9 more .remove() calls without validation - all can panic!
        }
    }
}
```

#### Solution

**Step 1: Add Safe Helpers to tcp.rs**

Add these functions right after the constants section (around line 76):

```rust
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
fn safe_pop_validated(data: &mut Vec<u8>, min: u8, max: u8, field_name: &str) -> anyhow::Result<u8> {
    let value = safe_pop(data)?;
    if value < min || value > max {
        anyhow::bail!(
            "Field {} value {} out of range [{}, {}]",
            field_name, value, min, max
        );
    }
    Ok(value)
}
```

**Step 2: Update parse_driverstation_tcp() Header**

**BEFORE:**
```rust
pub fn parse_driverstation_tcp(mut data: Vec<u8>) -> anyhow::Result<FromDS> {
    let size_upper = (data.remove(0) as u16) << 8;
    let size_lower = data.remove(0) as u16;
    let id = data.remove(0);
```

**AFTER:**
```rust
pub fn parse_driverstation_tcp(mut data: Vec<u8>) -> anyhow::Result<FromDS> {
    // Parse packet header with proper error handling
    let size = safe_pop_u16(&mut data)
        .context("Failed to parse packet size (need 2 bytes)")?;
    
    let id = safe_pop(&mut data)
        .context("Failed to parse packet ID (need 1 byte)")?;
    
    debug!(packet_size = size, packet_id = id, "Parsing packet");
```

**Step 3: Update Each Match Arm**

**0x15 Handler (UsageReport) - BEFORE:**
```rust
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
```

**AFTER:**
```rust
0x15 => {
    debug!("Parsing UsageReport packet");
    let team_num = safe_pop_u16(&mut data)
        .context("UsageReport: Failed to parse team_number (need 2 bytes)")?;
    
    let unknown = safe_pop(&mut data)
        .context("UsageReport: Failed to parse unknown field (need 1 byte)")?;
    
    let entry_data = EntryData { data: data };
    
    debug!(team_num, unknown, "UsageReport parsed successfully");
    tag_type = TagType::UsageReport(UsageReport {
        team_num,
        unknown,
        entry_data,
    })
}
```

**0x16 Handler (LogData) - BEFORE:**
```rust
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
```

**AFTER:**
```rust
0x16 => {
    debug!("Parsing LogData packet");
    
    let trip_time = safe_pop(&mut data)
        .context("LogData: Failed to parse trip_time")?;
    
    let lost_packets = safe_pop(&mut data)
        .context("LogData: Failed to parse lost_packets")?;
    
    let battery_xx = safe_pop(&mut data)
        .context("LogData: Failed to parse battery_xx")? as u16;
    
    let battery_yy = safe_pop(&mut data)
        .context("LogData: Failed to parse battery_yy")? as u16;
    
    let robot_status = safe_pop(&mut data)
        .context("LogData: Failed to parse robot_status")?;
    
    let can = safe_pop(&mut data)
        .context("LogData: Failed to parse can")?;
    
    let signal_db = safe_pop(&mut data)
        .context("LogData: Failed to parse signal_db")?;
    
    let bandwidth = safe_pop_u16(&mut data)
        .context("LogData: Failed to parse bandwidth")?;
    
    let battery = (battery_xx + battery_yy) / 256;
    
    debug!(
        trip_time, lost_packets, battery, robot_status, can, signal_db, bandwidth,
        "LogData parsed successfully"
    );
    
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
```

**0x18 Handler (TeamNumber) - BEFORE:**
```rust
0x18 => {
    let team_number_high = (data.remove(0) as u16) << 8;
    let team_number_low = data.remove(0) as u16;
    tag_type = TagType::TeamNumber(TeamNumber {
        team_number: team_number_high | team_number_low,
    })
}
```

**AFTER:**
```rust
0x18 => {
    debug!("Parsing TeamNumber packet");
    
    let team_number = safe_pop_u16(&mut data)
        .context("TeamNumber: Failed to parse team_number")?;
    
    debug!(team_number, "TeamNumber parsed successfully");
    
    tag_type = TagType::TeamNumber(TeamNumber {
        team_number,
    })
}
```

#### Testing
```bash
# These should now compile without panics
cargo build

# Test with malformed packets
cargo run
# Send: [0x00] (only 1 byte instead of 3) → Error logged, connection stays alive
# Send: [] (empty) → Error logged, connection stays alive
```

---

### P1.1.2: UDP Packet Creation - Eliminate .unwrap()

#### Problem
In `udp.rs:53-54`, `.unwrap()` is used on array access:

```rust
let period = data.control_mode.get(0).unwrap();  // 🔴 Panic if no index 0
let enabled = data.control_mode.get(1).unwrap(); // 🔴 Panic if no index 1
```

#### Solution

**BEFORE (udp.rs:45-70):**
```rust
pub fn create_udp_packet(data: &DriverstationConnection) -> Vec<u8> {
    let mut packet_data: Vec<u8> = Vec::new();

    packet_data.push(0x0);
    packet_data.push(0x0);
    packet_data.push(0x0);

    let mut control_byte: u8 = 0x00;
    let period = data.control_mode.get(0).unwrap();      // 🔴 UNWRAP!
    let enabled = data.control_mode.get(1).unwrap();     // 🔴 UNWRAP!

    match period {
        ControlMode::Teleop => control_byte = control_byte | 0b0000_0000,
        ControlMode::Test => control_byte = control_byte | 0b0000_0001,
        ControlMode::Autonomous => control_byte = control_byte | 0b0000_0010,
        _ => control_byte = control_byte | 0,
    };

    match enabled {
        ControlMode::Enabled => control_byte = control_byte | 0b0000_0100,
        ControlMode::Disabled => control_byte = control_byte | 0b0000_0000,
        _ => control_byte = control_byte | 0,
    };
    // ... rest of function
}
```

**AFTER:**
```rust
pub fn create_udp_packet(data: &DriverstationConnection) -> Vec<u8> {
    let mut packet_data: Vec<u8> = Vec::new();

    packet_data.push(0x0);
    packet_data.push(0x0);
    packet_data.push(0x0);

    let mut control_byte: u8 = 0x00;
    
    // Safe array access - use direct indexing for fixed-size array
    // control_mode is defined as [ControlMode; 2], so indices 0 and 1 are always valid
    let period = &data.control_mode[0];      // ✅ Safe: fixed-size array
    let enabled = &data.control_mode[1];     // ✅ Safe: fixed-size array

    match period {
        ControlMode::Teleop => control_byte = control_byte | 0b0000_0000,
        ControlMode::Test => control_byte = control_byte | 0b0000_0001,
        ControlMode::Autonomous => control_byte = control_byte | 0b0000_0010,
        _ => control_byte = control_byte | 0,
    };

    match enabled {
        ControlMode::Enabled => control_byte = control_byte | 0b0000_0100,
        ControlMode::Disabled => control_byte = control_byte | 0b0000_0000,
        _ => control_byte = control_byte | 0,
    };
    // ... rest of function
}
```

Add a documentation comment in the DriverstationConnection struct definition (in fms.rs or mod.rs):

```rust
/// Represents a single driverstation connection
pub struct DriverstationConnection {
    // ... other fields
    
    /// Robot control mode: [period_mode, enabled_state]
    /// Index 0: ControlMode::Teleop, Test, or Autonomous
    /// Index 1: ControlMode::Enabled or Disabled
    /// SAFETY: Must always be exactly 2 elements - enforced by type system
    pub control_mode: [ControlMode; 2],
}
```

---

### P1.1.3: Mutex Lock Error Handling

#### Problem
Mutex lock failures silently return instead of propagating errors:

```rust
match fms.lock() {
    Ok(lock) => { /* use lock */ }
    Err(e) => {
        eprintln!("Failed to acquire FMS lock: {e}");
        return; // 🔴 Silent failure - caller doesn't know error happened!
    }
}
```

#### Solution

**Step 1: Update Function Signatures to Return Result**

**In tcp.rs:140 - BEFORE:**
```rust
pub async fn ds_tcp_listener(
    mut socket: TcpStream,
    addr: SocketAddr,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>
) {
    // No return type - errors disappear!
```

**AFTER:**
```rust
pub async fn ds_tcp_listener(
    mut socket: TcpStream,
    addr: SocketAddr,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>
) -> anyhow::Result<()> {
    // Returns Result - errors can be caught and logged
```

**In driverstation_connection.rs - BEFORE:**
```rust
pub async fn new_driverstation(
    team_number: u16,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>,
) {
    // No return type
```

**AFTER:**
```rust
pub async fn new_driverstation(
    team_number: u16,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>,
) -> anyhow::Result<()> {
    // Returns Result
```

**Step 2: Replace Mutex Error Patterns**

**Pattern everywhere - BEFORE:**
```rust
match fms.lock() {
    Ok(mut fms_lock) => {
        // Do something with lock
        fms_lock.driverstations.push(ds);
    }
    Err(e) => {
        eprintln!("Poisoned FMS lock: {e}");
        return; // Silent failure
    }
}
```

**AFTER:**
```rust
{
    let mut fms_lock = fms.lock()
        .map_err(|e| anyhow::anyhow!(
            "FMS lock poisoned - possible panic in critical section: {}",
            e
        ))?;
    
    // Do something with lock
    fms_lock.driverstations.push(ds);
    
} // Lock automatically released here
```

**Complete example in tcp.rs (packet handling section) - BEFORE:**
```rust
pub async fn ds_tcp_listener(mut socket: TcpStream, addr: SocketAddr, shared_udp_socket: Arc<UdpSocket>, fms: Arc<Mutex<FMS>>) {
    loop {
        let mut buf = [0; 1024];
        match socket.read(&mut buf).await {
            Ok(n) => {
                let packet_data = buf[0..n].to_vec();
                if let Ok(parsed_packet) = parse_driverstation_tcp(packet_data) {
                    match parsed_packet.tag {
                        TagType::TeamNumber(tn) => {
                            match fms.lock() {
                                Ok(mut fms_lock) => {
                                    fms_lock.driverstations.push(DriverstationConnection::new(tn.team_number));
                                }
                                Err(e) => {
                                    eprintln!("Poisoned FMS lock: {e}");
                                    return; // ERROR: Silent exit!
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(_) => break,
        }
    }
}
```

**AFTER:**
```rust
pub async fn ds_tcp_listener(
    mut socket: TcpStream,
    addr: SocketAddr,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>
) -> anyhow::Result<()> {
    info!(remote_addr = %addr, "TCP listener started");
    
    loop {
        let mut buf = [0; 1024];
        match socket.read(&mut buf).await {
            Ok(n) => {
                let packet_data = buf[0..n].to_vec();
                
                match parse_driverstation_tcp(packet_data) {
                    Ok(parsed_packet) => {
                        match parsed_packet.tag {
                            TagType::TeamNumber(tn) => {
                                // Use ? operator to propagate lock errors
                                let mut fms_lock = fms.lock()
                                    .map_err(|e| anyhow::anyhow!(
                                        "FMS lock poisoned - possible panic in critical section: {}",
                                        e
                                    ))?;
                                
                                fms_lock.driverstations.push(
                                    DriverstationConnection::new(tn.team_number)
                                );
                                
                                info!(
                                    team_number = tn.team_number,
                                    "New driverstation registered"
                                );
                                
                                drop(fms_lock); // Explicitly release lock
                                
                                tokio::spawn(new_driverstation(
                                    tn.team_number,
                                    shared_udp_socket.clone(),
                                    fms.clone(),
                                ));
                            }
                            _ => {
                                debug!("Ignoring packet type: {:?}", parsed_packet.tag_id);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            remote_addr = %addr,
                            error = %e,
                            "Failed to parse TCP packet"
                        );
                    }
                }
            }
            Err(e) => {
                warn!(remote_addr = %addr, error = %e, "TCP read error - closing connection");
                break;
            }
        }
    }
    
    info!(remote_addr = %addr, "TCP listener exiting");
    Ok(())
}
```

**Step 3: Update Call Sites in main.rs - BEFORE:**
```rust
tokio::spawn(ds_tcp_listener(
    socket,
    addr,
    shared_udp_socket.clone(),
    fms.clone(),
));
```

**AFTER:**
```rust
tokio::spawn(async move {
    match ds_tcp_listener(
        socket,
        addr,
        shared_udp_socket.clone(),
        fms.clone(),
    ).await {
        Ok(_) => {
            debug!(remote_addr = %addr, "TCP listener completed normally");
        }
        Err(e) => {
            error!(remote_addr = %addr, error = %e, "TCP listener error");
        }
    }
});
```

---

### P1.1.4: Initialize Structured Logging

#### Problem
`tracing` crate is in Cargo.toml but never initialized. Code uses `println!()` and `eprintln!()`.

#### Solution

**Step 1: Update Cargo.toml dependencies (if needed)**

Check that `Cargo.toml` has:
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
# ... other deps
```

**Step 2: Initialize Tracing in main.rs**

**BEFORE (main.rs:1-30):**
```rust
use std::sync::{Arc, Mutex};

use actix_web::{App, HttpServer, web};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket, tcp},
};

use anyhow::{Context, Ok};

pub mod driverstation_comms;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ds_listener = TcpListener::bind(TCP_LISTENER_PORT)
        .await
        .context("Failed to open TCP listener server")?;
    println!("Spawned ds_listner server at port {}", TCP_LISTENER_PORT);
```

**AFTER:**
```rust
use std::sync::{Arc, Mutex};

use actix_web::{App, HttpServer, web};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket, tcp},
};
use tracing::{debug, info, warn, error};

use anyhow::{Context, Ok};

pub mod driverstation_comms;

const TCP_LISTENER_PORT: &str = "127.0.0.1:8080";
const HTTP_SERVER_PORT: &str = "127.0.0.1:2000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)          // Show module names in logs
        .with_thread_ids(true)      // Include thread ID
        .with_line_number(true)     // Include source line numbers
        .with_writer(std::io::stderr)  // Write to stderr
        .init();
    
    info!("🚀 FMS Server Starting");
    info!(tcp_port = TCP_LISTENER_PORT, http_port = HTTP_SERVER_PORT, "Configuration");

    let ds_listener = TcpListener::bind(TCP_LISTENER_PORT)
        .await
        .context("Failed to open TCP listener server")?;
    info!("✅ TCP listener bound to {}", TCP_LISTENER_PORT);
```

**Step 3: Replace All println! and eprintln! Throughout Codebase**

**Pattern - BEFORE:**
```rust
println!("Something happened");
eprintln!("Error occurred: {}", e);
println!("Team: {}, Status: {}", team_num, status);
```

**AFTER:**
```rust
info!("Something happened");
error!(error = ?e, "Error occurred");
debug!(team_num, status, "Status updated");
```

**Common replacements:**

| Before | After | Level |
|--------|-------|-------|
| `println!("msg")` | `info!("msg")` | INFO |
| `eprintln!("err: {}", e)` | `error!(error = ?e, "err")` | ERROR |
| `println!("debug: {}", val)` | `debug!(val, "debug")` | DEBUG |
| `eprintln!("warning")` | `warn!("warning")` | WARN |

**Full example in tcp.rs - BEFORE:**
```rust
pub async fn ds_tcp_listener(...) {
    loop {
        // ...
        match parse_driverstation_tcp(packet_data) {
            Ok(parsed) => {
                eprintln!("not yet implemented: {:?}", parsed.tag_type);
            }
            Err(e) => {
                eprintln!("Parse error: {}", e);
            }
        }
    }
}
```

**AFTER:**
```rust
use tracing::{debug, info, warn, error};

pub async fn ds_tcp_listener(...) -> anyhow::Result<()> {
    info!(remote_addr = %addr, "TCP listener started");
    
    loop {
        // ...
        match parse_driverstation_tcp(packet_data) {
            Ok(parsed) => {
                warn!(tag_id = parsed.tag_id, "Packet type not yet implemented");
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse packet");
            }
        }
    }
    
    info!(remote_addr = %addr, "TCP listener exiting");
    Ok(())
}
```

**In driverstation_connection.rs - BEFORE:**
```rust
println!(
    "new driverstation control thread created for team {}",
    team_number
);
// ...
eprintln!("Failed to send UDP packet to {}: {e}", driverstation_ip);
```

**AFTER:**
```rust
info!(team_number, "New driverstation control thread created");
// ...
error!(driverstation_ip, error = ?e, "Failed to send UDP packet");
```

---

## P1.2: TCP Protocol Completion

### P1.2.1: Implement Version Tag Handler (0x00-0x07)

#### Problem
Version packet handler is stubbed:

```rust
0x00..=0x07 => {
    // TODO Version stuffs
}
```

#### Solution

**Step 1: Add Version Data Structure**

In `driverstation_comms/mod.rs`, add:

```rust
/// Version information from driverstation
#[derive(Debug, Clone)]
pub struct Version {
    pub tag_id: u8,
    pub version_bytes: Vec<u8>,
}

/// Update the TagType enum to include Version
pub enum TagType {
    DSPing(DSPing),
    UsageReport(UsageReport),
    LogData(LogData),
    ErrorEventData(ErrorEventData),
    TeamNumber(TeamNumber),
    Version(Version),  // ← Add this variant
}
```

**Step 2: Implement Handler**

In `tcp.rs` (replace the stubbed handler):

**BEFORE (tcp.rs:86-88):**
```rust
0x00..=0x07 => {
    // TODO Version stuffs
}
```

**AFTER:**
```rust
0x00..=0x07 => {
    debug!(tag_id = id, "Processing Version packet");
    
    // Version packet contains protocol version information
    // First byte after header typically indicates protocol version
    if let Some(protocol_version) = data.first() {
        debug!(protocol_version, "Driverstation protocol version");
    }
    
    // Store remaining version bytes for future use
    tag_type = TagType::Version(Version {
        tag_id: id,
        version_bytes: data.clone(),
    });
    
    info!(tag_id = id, "Version packet received and stored");
}
```

---

### P1.2.2: Implement Error/Event Data Handler (0x17)

#### Problem
Error/Event handler is stubbed:

```rust
0x17 => {
    // TODO: Error and event data
}
```

#### Solution

**Step 1: Add ErrorEventData Structure**

In `driverstation_comms/mod.rs`:

```rust
/// Error or event data from driverstation
#[derive(Debug, Clone)]
pub struct ErrorEventData {
    pub error_flags: u8,
    pub error_message: Vec<u8>,
}

/// Update TagType enum
pub enum TagType {
    // ... existing variants
    ErrorEventData(ErrorEventData),  // ← Add this
}
```

**Step 2: Implement Handler**

In `tcp.rs`:

**BEFORE (tcp.rs:120-122):**
```rust
0x17 => {
    // TODO: Error and event data
}
```

**AFTER:**
```rust
0x17 => {
    debug!(tag_id = id, "Processing Error/Event Data packet");
    
    let error_flags = safe_pop(&mut data)
        .context("ErrorEventData: Failed to parse error_flags")?;
    
    let error_message = data; // Remaining bytes are the error message
    
    // Decode error flags
    let is_warning = (error_flags & 0x80) != 0;
    let error_category = error_flags & 0x7F;
    
    warn!(
        tag_id = id,
        error_flags = error_flags,
        is_warning,
        error_category,
        message_len = error_message.len(),
        "Driverstation error/event reported"
    );
    
    tag_type = TagType::ErrorEventData(ErrorEventData {
        error_flags,
        error_message,
    });
}
```

---

### P1.2.3: Team Number Parsing Robustness

#### Problem
Team number string slicing can panic if format is unexpected:

```rust
let team_number_string = format!("{}", team_number);
let upper_team_numbers = if team_number_string.chars().count() > 5 {
    &team_number_string[0..3]  // 🔴 Can panic!
} else {
    &team_number_string[0..2]  // 🔴 Can panic if empty!
};
```

#### Solution

**Step 1: Add IP Parsing Helper**

In `driverstation_connection.rs`, add before `new_driverstation()`:

```rust
/// Parse FIRST team number into IP octets
/// 
/// FIRST robotics uses team number to derive team IP:
/// Team XXYY → IP 10.XX.YY.5
/// 
/// Examples:
/// - Team 0001 → 10.00.01.5
/// - Team 0254 → 10.02.54.5
/// - Team 1234 → 10.12.34.5
fn parse_team_ip_octets(team_number: u16) -> anyhow::Result<(String, String)> {
    // Pad team number to 4 digits
    let padded = format!("{:04}", team_number);
    
    if padded.len() != 4 {
        anyhow::bail!(
            "Team number {} produced invalid format: {} (len={})",
            team_number,
            padded,
            padded.len()
        );
    }
    
    // Split XXYY into XX and YY
    let upper = padded[0..2].to_string();
    let lower = padded[2..4].to_string();
    
    debug!(team_number, upper, lower, "Team number parsed to IP octets");
    
    Ok((upper, lower))
}
```

**Step 2: Update new_driverstation() to use Helper and Return Result**

**BEFORE:**
```rust
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
        // ... rest of function
    }
}
```

**AFTER:**
```rust
pub async fn new_driverstation(
    team_number: u16,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>,
) -> anyhow::Result<()> {
    info!(team_number, "New driverstation control thread created");

    let (upper_team_numbers, lower_team_numbers) = parse_team_ip_octets(team_number)
        .context(format!("Failed to parse IP octets for team {}", team_number))?;

    loop {
        // ... rest of function (updated below)
    }
}
```

**Step 3: Update UDP Send Loop with Proper Error Handling**

**BEFORE (lines 68-115):**
```rust
loop {
    let mut driverstation_ip: String = "".to_string();
    let mut driverstation_connection: DriverstationConnection =
        DriverstationConnection::new(team_number);
    let mut found_driverstation = false;

    {
        match fms.lock() {
            Ok(fms_lock) => {
                let driverstation_list = fms_lock.driverstations.clone();

                if let Some(driverstation_connection_position) = driverstation_list
                    .iter()
                    .position(|driverstation| driverstation.team_number == team_number)
                {
                    if let Some(ds_conn) = fms_lock.driverstations.get(driverstation_connection_position) {
                        driverstation_connection = ds_conn.clone();
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
                    println!("could not find driverstation connection in driverstation vec");
                }
            }
            Err(e) => {
                eprintln!("Failed to acquire FMS lock: {e}");
                return;
            }
        }
    }
    if found_driverstation {
        match shared_udp_socket
            .send_to(
                &create_udp_packet(&driverstation_connection),
                &driverstation_ip,
            )
            .await
        {
            Ok(_) => println!("sent udp packet to {}", &driverstation_ip),
            Err(e) => {
                eprintln!("Failed to send UDP packet to {}: {e}", driverstation_ip);
            }
        }
    } else {
        println!("could not find driverstation connection for {team_number}; terminating udp connection");
        return;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
}
```

**AFTER:**
```rust
let mut consecutive_send_failures: u32 = 0;
const MAX_CONSECUTIVE_FAILURES: u32 = 10;

loop {
    // Acquire lock and get driverstation state
    let driverstation_ip: String = {
        let fms_lock = fms.lock()
            .map_err(|e| anyhow::anyhow!(
                "FMS lock poisoned while getting driverstation state: {}",
                e
            ))?;

        // Find this team in the driverstation list
        let driverstation_position = fms_lock.driverstations
            .iter()
            .position(|ds| ds.team_number == team_number);

        match driverstation_position {
            Some(pos) => {
                if let Some(ds_conn) = fms_lock.driverstations.get(pos) {
                    driverstation_connection = ds_conn.clone();
                    
                    // Generate IP address based on control type
                    match driverstation_connection.ds_control {
                        DSControl::FMSFull => {
                            format!("10.{}.{}.5:1120", upper_team_numbers, lower_team_numbers)
                        }
                        DSControl::FMSPartial => {
                            format!("10.{}.{}.5:1121", upper_team_numbers, lower_team_numbers)
                        }
                        DSControl::Uncontrolled => {
                            format!("0.0.0.0:1120")
                        }
                    }
                } else {
                    anyhow::bail!(
                        "Driverstation position {} not found in list (possible race condition)",
                        pos
                    );
                }
            }
            None => {
                warn!(team_number, "Driverstation not found in FMS registry");
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue; // Try again next cycle
            }
        }
    }; // Lock released here

    // Send UDP packet
    match shared_udp_socket
        .send_to(
            &create_udp_packet(&driverstation_connection),
            &driverstation_ip,
        )
        .await
    {
        Ok(bytes_sent) => {
            debug!(
                team_number,
                driverstation_ip,
                bytes_sent,
                "UDP packet sent successfully"
            );
            consecutive_send_failures = 0; // Reset failure counter
        }
        Err(e) => {
            consecutive_send_failures += 1;
            error!(
                team_number,
                driverstation_ip,
                error = ?e,
                attempt = consecutive_send_failures,
                "Failed to send UDP packet"
            );

            // Stop trying after max failures
            if consecutive_send_failures >= MAX_CONSECUTIVE_FAILURES {
                error!(
                    team_number,
                    max_failures = MAX_CONSECUTIVE_FAILURES,
                    "Max consecutive send failures reached - exiting UDP sender"
                );
                anyhow::bail!(
                    "UDP sender for team {} exceeded max failures",
                    team_number
                );
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
}

Ok(())
```

---

### P1.2.4: Connection Timeout Handling

#### Problem
TCP connections have no idle timeout - they can hang indefinitely.

#### Solution

**In tcp.rs, update ds_tcp_listener():**

**BEFORE:**
```rust
pub async fn ds_tcp_listener(mut socket: TcpStream, addr: SocketAddr, shared_udp_socket: Arc<UdpSocket>, fms: Arc<Mutex<FMS>>) {
    loop {
        let mut buf = [0; 1024];
        match socket.read(&mut buf).await {
            Ok(0) => {
                // Socket closed
                break;
            }
            Ok(n) => {
                // ... handle packet
            }
            Err(_) => {
                break;
            }
        }
    }
}
```

**AFTER:**
```rust
const CONNECTION_IDLE_TIMEOUT_SECS: u64 = 30;

pub async fn ds_tcp_listener(
    mut socket: TcpStream,
    addr: SocketAddr,
    shared_udp_socket: Arc<UdpSocket>,
    fms: Arc<Mutex<FMS>>
) -> anyhow::Result<()> {
    info!(remote_addr = %addr, timeout_secs = CONNECTION_IDLE_TIMEOUT_SECS, "TCP listener started");

    loop {
        let mut buf = [0; 1024];

        // Wrap read in timeout: disconnect if no data for CONNECTION_IDLE_TIMEOUT_SECS
        let read_timeout = tokio::time::Duration::from_secs(CONNECTION_IDLE_TIMEOUT_SECS);
        
        let read_result = tokio::time::timeout(
            read_timeout,
            socket.read(&mut buf),
        ).await;

        match read_result {
            // Timeout occurred - no data received before timeout
            Err(_timeout) => {
                warn!(
                    remote_addr = %addr,
                    timeout_secs = CONNECTION_IDLE_TIMEOUT_SECS,
                    "Connection idle timeout exceeded - disconnecting"
                );
                break; // Exit listener loop
            }

            // Socket closed by remote (read returns 0 bytes)
            Ok(Ok(0)) => {
                info!(remote_addr = %addr, "Connection closed by remote");
                break;
            }

            // Data received successfully
            Ok(Ok(n)) => {
                debug!(
                    remote_addr = %addr,
                    bytes_received = n,
                    "TCP packet received"
                );

                let packet_data = buf[0..n].to_vec();

                // Parse and handle packet
                match parse_driverstation_tcp(packet_data) {
                    Ok(parsed_packet) => {
                        match parsed_packet.tag {
                            TagType::TeamNumber(tn) => {
                                let mut fms_lock = fms.lock()
                                    .map_err(|e| anyhow::anyhow!(
                                        "FMS lock poisoned: {}",
                                        e
                                    ))?;

                                fms_lock.driverstations.push(
                                    DriverstationConnection::new(tn.team_number)
                                );

                                info!(
                                    team_number = tn.team_number,
                                    "New driverstation registered"
                                );

                                drop(fms_lock);

                                // Spawn UDP sender task for this team
                                let fms_clone = fms.clone();
                                let udp_clone = shared_udp_socket.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = new_driverstation(
                                        tn.team_number,
                                        udp_clone,
                                        fms_clone,
                                    ).await {
                                        error!(
                                            team_number = tn.team_number,
                                            error = %e,
                                            "UDP sender exited with error"
                                        );
                                    }
                                });
                            }
                            _ => {
                                debug!(
                                    tag_id = parsed_packet.tag_id,
                                    "Packet type processed"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(
                            remote_addr = %addr,
                            error = %e,
                            "Failed to parse TCP packet"
                        );
                    }
                }
            }

            // Socket read error
            Ok(Err(e)) => {
                warn!(
                    remote_addr = %addr,
                    error = %e,
                    "TCP read error - closing connection"
                );
                break;
            }
        }
    }

    info!(remote_addr = %addr, "TCP listener exiting");
    Ok(())
}
```

---

## P1.3: Build & Testing Infrastructure

### Setting Up Clippy and Format Checks

Add to your development workflow:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Run clippy linter
cargo clippy --all-targets

# Build for release
cargo build --release
```

### Manual Testing Checklist

After completing all Phase 1 changes:

```bash
# 1. Build without warnings
cargo build --release
# Should complete with no errors or warnings

# 2. Run clippy
cargo clippy --all-targets
# Should pass without warnings

# 3. Check format
cargo fmt --check
# Should pass

# 4. Run the server
cargo run
# Should start and show:
# - Tracing subscriber initialized
# - TCP listener bound message
# - All future logs with proper formatting

# 5. Test with mock driverstation (in another terminal)
# Uncomment the test client in main.rs or use:
# echo -n -e '\x00\xff\x18\x0e\xb7' | nc 127.0.0.1 8080
# Should see: TeamNumber packet parsed, driverstation registered

# 6. Test malformed packets
# echo -n -e '\x00' | nc 127.0.0.1 8080
# Should see: error logged, connection still alive (no crash)

# 7. Test timeout (leave connection idle for 30+ seconds)
# Server should log timeout and disconnect (no hanging connections)
```

---

## Phase 1 Completion Checklist

- [x] Created `safe_pop()` and `safe_pop_u16()` helpers
- [x] Updated all `.remove()` calls in `parse_driverstation_tcp()`
- [x] Replaced `.unwrap()` in `udp.rs` with direct indexing
- [x] Made `ds_tcp_listener()` return `Result<()>`
- [x] Made `new_driverstation()` return `Result<()>`
- [x] Replaced all `match fms.lock()` with error propagation
- [x] Initialized `tracing_subscriber` in `main.rs`
- [x] Replaced all `println!()` with `info!()`/`debug!()`
- [x] Replaced all `eprintln!()` with `warn!()`/`error!()`
- [ ] Implemented Version tag handler (0x00-0x07)
- [ ] Implemented ErrorEventData handler (0x17)
- [x] Added `parse_team_ip_octets()` helper
- [ ] Added connection timeout handling
- [ ] `cargo build` passes without warnings
- [ ] `cargo clippy --all-targets` passes
- [ ] Manual testing passes (malformed packets, timeouts, logging)

---

## Summary

Phase 1 transforms the codebase from crash-prone to production-grade by:
1. Eliminating all panic vectors
2. Adding proper error propagation and logging
3. Completing protocol implementation
4. Implementing defensive timeouts

This creates a solid foundation for Phase 2 (Safety features) and beyond.

