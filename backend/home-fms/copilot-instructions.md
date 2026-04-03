# Copilot Instructions for home-fms

## Build, Test, and Run Commands

```bash
# Build the project
cargo build --release

# Run the server locally
cargo run

# Format code (standard Rust)
cargo fmt

# Lint with Clippy
cargo clippy --all-targets
```

**Note:** The project currently has no tests. When adding functionality, include unit tests in the module files and consider integration tests for protocol parsing.

## High-Level Architecture

**home-fms** is an async networking server for the FIRST Robotics Field Management System. It manages communication between driverstations and robots using a two-protocol model:

- **Inbound (TCP):** Driverstations connect to port 8080 and send binary packets with status updates (team number, control mode, battery voltage, logs, errors)
- **Outbound (UDP):** The FMS sends control commands to robots every 500ms via UDP port 8080, using team IP convention: team 1234 → 10.12.34.5
- **HTTP Status:** Health check endpoint at localhost:2000/status

**Core Design:**
```
TCP Listener → Parse packets → Update shared FMS state (Arc<Mutex<FMS>>)
                                        ↓
                                   Per-team UDP workers → Send control commands to robots
```

The FMS maintains a central registry of:
- Active driverstation connections
- Allowed teams for the current match
- Per-team control state (enabled/disabled, mode, alliance station)

## Key Conventions

### Async Model
- **Tokio-based:** Every driverstation connection runs in its own spawned task (`tokio::spawn()`)
- **Shared state:** The `FMS` struct is wrapped in `Arc<Mutex<FMS>>` and passed to all tasks
- **Minimize lock contention:** Acquire the Mutex, make changes quickly, and release—don't hold locks across await points

### Protocol Implementation Pattern
TCP packet handlers follow a tag-based system. When adding a new packet type:
1. Add the tag ID constant (e.g., `0x08`)
2. Implement parsing logic in `parse_driverstation_tcp()`
3. Update the FMS state if needed
4. Log errors with context (packet content, team number)

Example:
```rust
0x08 => {
    // Parse packet bytes starting at offset 1
    let value = u16::from_be_bytes([data[1], data[2]]);
    // Update state via fms.lock()
    let mut fms = fms.lock().await;
    fms.some_field = value;
}
```

### Module Organization
- **`tcp.rs`** – TCP packet parsing and protocol definitions
- **`udp.rs`** – UDP packet creation and robot control commands
- **`driverstation_connection.rs`** – DriverStation connection state and handlers
- **`fms.rs`** – Central FMS state and logic
- **`main.rs`** – Entry point, socket initialization, async runtime setup

### Naming Conventions
- `ds_*` prefix for driverstation-related items
- `fms_*` prefix for FMS-level operations
- `parse_*` for parsing functions
- `create_*` for packet creation functions

### Error Handling
Use `anyhow::Result<T>` for error propagation. Log errors with context before returning, so debugging is easier downstream.

## Known Incomplete Implementations

These areas need work and may require protocol documentation:
- TCP packet types 0x00 (Version), 0x17 (ErrorEventData)
- Challenge response mechanism
- Battery voltage parsing and unit validation
- Graceful shutdown mechanism
- Connection limits and rate limiting

When working on these, refer to the FIRST FMS protocol specification if available, or check the commented test code in `main.rs` for packet format hints.
