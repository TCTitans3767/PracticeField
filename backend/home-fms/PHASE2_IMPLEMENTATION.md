# Phase 2: Safety Infrastructure - E-Stop & Graceful Degradation

**Objective:** Implement emergency stop mechanisms, prevent stale commands, enforce state machines, and ensure safe operation.

**Duration:** ~2-2.5 hours  
**Status:** Depends on Phase 1 completion  
**Depends On:** P1.1, P1.2 (all error handling & protocol)

---

## Overview

Phase 2 focuses on safety-critical functionality. The system must be able to:
- Stop all robots immediately via global e-stop
- Stop individual robots via per-robot e-stop
- Never send commands to disconnected driverstations
- Enforce valid state transitions
- Gracefully degrade when subsystems fail

This phase adds the safety layer that prevents robots from entering dangerous states.

---

## P2.1: E-Stop System (Per-Robot & Global)

### P2.1.1: Define E-Stop State

#### Problem
Currently, there's no e-stop mechanism. Robots cannot be emergency-stopped once enabled.

#### Solution

**Step 1: Update DriverstationConnection Structure**

In `driverstation_comms/mod.rs` or `fms.rs`:

**BEFORE:**
```rust
#[derive(Debug, Clone)]
pub struct DriverstationConnection {
    pub team_number: u16,
    pub ds_control: DSControl,
    pub alliance_station: u8,
    pub control_mode: [ControlMode; 2],
}
```

**AFTER:**
```rust
/// E-stop state for a robot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EStopState {
    /// Robot is operating normally (e-stop not active)
    Active,
    /// E-stop has been triggered (robot should be disabled)
    Triggered,
    /// E-stop cleared and ready to arm (manual reset required)
    Cleared,
}

#[derive(Debug, Clone)]
pub struct DriverstationConnection {
    pub team_number: u16,
    pub ds_control: DSControl,
    pub alliance_station: u8,
    pub control_mode: [ControlMode; 2],
    
    /// E-stop state: Active means e-stop can engage, Triggered means it's active
    pub estop_state: EStopState,
    
    /// Timestamp of last TCP heartbeat (used for disconnect detection)
    pub last_heartbeat: std::time::Instant,
}

impl Default for DriverstationConnection {
    fn default() -> Self {
        Self {
            team_number: 0,
            ds_control: DSControl::Uncontrolled,
            alliance_station: 0,
            control_mode: [ControlMode::Disabled, ControlMode::Disabled],
            estop_state: EStopState::Active,
            last_heartbeat: std::time::Instant::now(),
        }
    }
}

impl DriverstationConnection {
    pub fn new(team_number: u16) -> Self {
        Self {
            team_number,
            last_heartbeat: std::time::Instant::now(),
            ..Default::default()
        }
    }
}
```

**Step 2: Add FMS-Level E-Stop Tracking**

Update `FMS` struct:

**BEFORE:**
```rust
pub struct FMS {
    pub driverstations: Vec<DriverstationConnection>,
    pub allowed_driverstations: Vec<u16>,
}
```

**AFTER:**
```rust
/// Global e-stop state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalEStopState {
    /// Normal operation (e-stop not engaged)
    Normal,
    /// Global e-stop triggered (all robots disabled)
    TriggeredGlobal,
    /// Global e-stop cleared (manual reset required)
    ClearedGlobal,
}

pub struct FMS {
    pub driverstations: Vec<DriverstationConnection>,
    pub allowed_driverstations: Vec<u16>,
    
    /// Global e-stop state affects all robots
    pub global_estop: GlobalEStopState,
    
    /// Timestamp when global e-stop was triggered (for logging/audit trail)
    pub global_estop_triggered_at: Option<std::time::Instant>,
}

impl Default for FMS {
    fn default() -> Self {
        Self {
            driverstations: Vec::new(),
            allowed_driverstations: Vec::new(),
            global_estop: GlobalEStopState::Normal,
            global_estop_triggered_at: None,
        }
    }
}
```

**Step 3: Helper Methods**

Add these methods to impl blocks:

```rust
impl DriverstationConnection {
    /// Check if robot should be disabled due to e-stop
    pub fn is_estop_active(&self) -> bool {
        self.estop_state == EStopState::Triggered
    }

    /// Trigger e-stop for this robot
    pub fn trigger_estop(&mut self) {
        if self.estop_state != EStopState::Triggered {
            warn!(team_number = self.team_number, "E-stop triggered for robot");
            self.estop_state = EStopState::Triggered;
            // Disable robot immediately
            self.control_mode = [ControlMode::Disabled, ControlMode::Disabled];
        }
    }

    /// Clear e-stop (allows re-arming)
    pub fn clear_estop(&mut self) {
        self.estop_state = EStopState::Cleared;
        info!(team_number = self.team_number, "E-stop cleared for robot");
    }

    /// Arm e-stop (return to Normal state)
    pub fn arm_estop(&mut self) {
        self.estop_state = EStopState::Active;
        debug!(team_number = self.team_number, "E-stop armed for robot");
    }

    /// Update heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = std::time::Instant::now();
    }

    /// Check if connection is idle (no heartbeat for > timeout)
    pub fn is_idle(&self, timeout_secs: u64) -> bool {
        self.last_heartbeat.elapsed().as_secs() > timeout_secs
    }
}

impl FMS {
    /// Trigger global e-stop (all robots)
    pub fn trigger_global_estop(&mut self) {
        if self.global_estop != GlobalEStopState::TriggeredGlobal {
            error!("🛑 GLOBAL E-STOP TRIGGERED - Disabling all robots");
            self.global_estop = GlobalEStopState::TriggeredGlobal;
            self.global_estop_triggered_at = Some(std::time::Instant::now());

            // Disable all robots immediately
            for ds in self.driverstations.iter_mut() {
                ds.control_mode = [ControlMode::Disabled, ControlMode::Disabled];
                warn!(
                    team_number = ds.team_number,
                    "Global e-stop: robot disabled"
                );
            }
        }
    }

    /// Clear global e-stop
    pub fn clear_global_estop(&mut self) {
        if self.global_estop == GlobalEStopState::TriggeredGlobal {
            warn!("Global e-stop cleared - manual reset required before re-arm");
            self.global_estop = GlobalEStopState::ClearedGlobal;
        }
    }

    /// Arm global e-stop (return to normal)
    pub fn arm_global_estop(&mut self) {
        self.global_estop = GlobalEStopState::Normal;
        self.global_estop_triggered_at = None;
        info!("Global e-stop armed - ready for normal operation");
    }

    /// Check if any e-stop is active (global or per-robot)
    pub fn is_any_estop_active(&self) -> bool {
        self.global_estop == GlobalEStopState::TriggeredGlobal
            || self.driverstations.iter().any(|ds| ds.is_estop_active())
    }
}
```

---

### P2.1.2: E-Stop Endpoints

#### Solution

Add to `main.rs` (in the `interface` module):

```rust
pub mod interface {
    use actix_web::{web, HttpResponse, Responder};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use crate::driverstation_comms::fms::FMS;
    use tracing::{info, warn, error};

    pub async fn status() -> impl Responder {
        HttpResponse::Ok().json(json!({
            "status": "operational",
            "version": "0.1.0"
        }))
    }

    /// Trigger per-robot e-stop
    pub async fn trigger_robot_estop(
        team_id: web::Path<u16>,
        fms: web::Data<Arc<Mutex<FMS>>>,
    ) -> impl Responder {
        let team_number = team_id.into_inner();

        match fms.lock() {
            Ok(mut fms_lock) => {
                if let Some(ds) = fms_lock.driverstations.iter_mut()
                    .find(|ds| ds.team_number == team_number)
                {
                    warn!(team_number, "E-stop triggered via API");
                    ds.trigger_estop();

                    HttpResponse::Ok().json(json!({
                        "success": true,
                        "message": format!("E-stop triggered for team {}", team_number),
                        "team": team_number,
                        "estop_state": "triggered"
                    }))
                } else {
                    HttpResponse::NotFound().json(json!({
                        "success": false,
                        "error": format!("Team {} not found", team_number),
                        "team": team_number
                    }))
                }
            }
            Err(e) => {
                error!(error = ?e, "Failed to acquire FMS lock in estop endpoint");
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": "FMS lock failed - server may be in unsafe state"
                }))
            }
        }
    }

    /// Clear per-robot e-stop (manual reset)
    pub async fn clear_robot_estop(
        team_id: web::Path<u16>,
        fms: web::Data<Arc<Mutex<FMS>>>,
    ) -> impl Responder {
        let team_number = team_id.into_inner();

        match fms.lock() {
            Ok(mut fms_lock) => {
                if let Some(ds) = fms_lock.driverstations.iter_mut()
                    .find(|ds| ds.team_number == team_number)
                {
                    info!(team_number, "E-stop cleared via API");
                    ds.clear_estop();

                    HttpResponse::Ok().json(json!({
                        "success": true,
                        "message": format!("E-stop cleared for team {}", team_number),
                        "team": team_number,
                        "estop_state": "cleared"
                    }))
                } else {
                    HttpResponse::NotFound().json(json!({
                        "success": false,
                        "error": format!("Team {} not found", team_number)
                    }))
                }
            }
            Err(e) => {
                error!(error = ?e, "Failed to acquire FMS lock");
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": "Server error"
                }))
            }
        }
    }

    /// Trigger global e-stop (all robots)
    pub async fn trigger_global_estop(
        fms: web::Data<Arc<Mutex<FMS>>>,
    ) -> impl Responder {
        match fms.lock() {
            Ok(mut fms_lock) => {
                error!("🛑 GLOBAL E-STOP TRIGGERED VIA API");
                fms_lock.trigger_global_estop();

                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "Global e-stop triggered - all robots disabled",
                    "estop_state": "triggered_global"
                }))
            }
            Err(e) => {
                error!(error = ?e, "Failed to acquire FMS lock for global estop");
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": "FMS lock failed"
                }))
            }
        }
    }

    /// Clear global e-stop (manual reset)
    pub async fn clear_global_estop(
        fms: web::Data<Arc<Mutex<FMS>>>,
    ) -> impl Responder {
        match fms.lock() {
            Ok(mut fms_lock) => {
                warn!("Global e-stop cleared via API");
                fms_lock.clear_global_estop();

                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "Global e-stop cleared",
                    "estop_state": "cleared_global"
                }))
            }
            Err(e) => {
                error!(error = ?e, "Failed to acquire FMS lock");
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": "Server error"
                }))
            }
        }
    }

    /// Get e-stop status for all robots
    pub async fn get_estop_status(
        fms: web::Data<Arc<Mutex<FMS>>>,
    ) -> impl Responder {
        match fms.lock() {
            Ok(fms_lock) => {
                let robots_status: Vec<_> = fms_lock.driverstations
                    .iter()
                    .map(|ds| json!({
                        "team": ds.team_number,
                        "estop_state": format!("{:?}", ds.estop_state)
                    }))
                    .collect();

                HttpResponse::Ok().json(json!({
                    "global_estop": format!("{:?}", fms_lock.global_estop),
                    "robots": robots_status
                }))
            }
            Err(e) => {
                error!(error = ?e, "Failed to acquire FMS lock");
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "error": "Server error"
                }))
            }
        }
    }
}
```

**Step 4: Register Routes in main.rs**

Update the HTTP server setup:

**BEFORE:**
```rust
let _ = HttpServer::new(|| App::new()
    .route("/status", web::get().to(interface::status)))
    .bind("127.0.0.1:2000")
    .context("failed to run web server")?
    .run()
    .await;
```

**AFTER:**
```rust
let fms_for_http = fms.clone(); // Clone for HTTP handler

let _ = HttpServer::new(move || {
    App::new()
        .app_data(web::Data::new(fms_for_http.clone()))
        // Status endpoints
        .route("/status", web::get().to(interface::status))
        // E-stop endpoints
        .route("/api/robots/{team}/estop", web::post().to(interface::trigger_robot_estop))
        .route("/api/robots/{team}/estop/clear", web::post().to(interface::clear_robot_estop))
        .route("/api/estop", web::post().to(interface::trigger_global_estop))
        .route("/api/estop/clear", web::post().to(interface::clear_global_estop))
        .route("/api/estop/status", web::get().to(interface::get_estop_status))
})
.bind(HTTP_SERVER_PORT)
.context("failed to run web server")?
.run()
.await;
```

---

## P2.2: Stale Command Prevention

### P2.2.1: Heartbeat Monitoring

#### Problem
Currently, when a driverstation disconnects, the FMS still sends UDP commands to the disconnected robot, potentially leaving the robot in a dangerous state.

#### Solution

**Step 1: Add Background Task for Disconnect Detection**

Create a new async function to monitor connections. Add to `tcp.rs` or create `monitor.rs`:

```rust
/// Background task that monitors driverstation connections
/// Disconnects any that haven't sent heartbeat in > timeout
pub async fn connection_monitor(
    fms: Arc<Mutex<FMS>>,
) -> anyhow::Result<()> {
    const CHECK_INTERVAL_MS: u64 = 100;
    const HEARTBEAT_TIMEOUT_SECS: u64 = 5;

    info!(
        check_interval_ms = CHECK_INTERVAL_MS,
        heartbeat_timeout_secs = HEARTBEAT_TIMEOUT_SECS,
        "Connection monitor started"
    );

    loop {
        tokio::time::sleep(Duration::from_millis(CHECK_INTERVAL_MS)).await;

        let (idle_teams, total_teams) = {
            let mut fms_lock = fms.lock()
                .map_err(|e| anyhow::anyhow!("FMS lock poisoned: {}", e))?;

            let mut idle_teams = Vec::new();

            // Check each connection for idle timeout
            for ds in fms_lock.driverstations.iter_mut() {
                if ds.is_idle(HEARTBEAT_TIMEOUT_SECS) {
                    warn!(
                        team_number = ds.team_number,
                        idle_secs = HEARTBEAT_TIMEOUT_SECS,
                        "Driverstation idle timeout - marking for disconnect"
                    );

                    // Disable robot immediately
                    ds.control_mode = [ControlMode::Disabled, ControlMode::Disabled];
                    idle_teams.push(ds.team_number);
                }
            }

            // Remove idle driverstations
            fms_lock.driverstations.retain(|ds| !idle_teams.contains(&ds.team_number));

            let remaining = fms_lock.driverstations.len();
            (idle_teams, remaining)
        }; // Lock released

        if !idle_teams.is_empty() {
            warn!(
                disconnected_teams = ?idle_teams,
                remaining_teams = total_teams,
                "Driverstations disconnected due to inactivity"
            );
        }
    }
}
```

**Step 2: Update Heartbeat on Packet Receive**

In `tcp.rs`, update the packet receive handler to update heartbeat:

**BEFORE:**
```rust
Ok(Ok(n)) => {
    debug!(remote_addr = %addr, bytes_received = n, "TCP packet received");
    let packet_data = buf[0..n].to_vec();
    
    match parse_driverstation_tcp(packet_data) {
        // ... handle packet
    }
}
```

**AFTER:**
```rust
Ok(Ok(n)) => {
    debug!(remote_addr = %addr, bytes_received = n, "TCP packet received");
    
    // Update heartbeat for all driverstations (indicating TCP connection is alive)
    {
        if let Ok(mut fms_lock) = fms.lock() {
            for ds in fms_lock.driverstations.iter_mut() {
                ds.update_heartbeat();
            }
        }
    }
    
    let packet_data = buf[0..n].to_vec();
    
    match parse_driverstation_tcp(packet_data) {
        // ... handle packet
    }
}
```

**Step 3: Spawn Monitor Task in main.rs**

**BEFORE:**
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... initialization
    
    tokio::spawn(tcp_listener(
        ds_listener,
        shared_udp_socket.clone(),
        fms.clone(),
    ));
```

**AFTER:**
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... initialization
    
    // Spawn TCP listener
    tokio::spawn(tcp_listener(
        ds_listener,
        shared_udp_socket.clone(),
        fms.clone(),
    ));

    // Spawn connection monitor task
    let fms_monitor = fms.clone();
    tokio::spawn(async move {
        if let Err(e) = connection_monitor(fms_monitor).await {
            error!(error = %e, "Connection monitor exited with error");
        }
    });
```

---

## P2.3: State Machine Enforcement

### Problem
Currently, there are no constraints on valid robot state transitions. A robot could go directly from Disconnected to Enabled, which shouldn't be allowed.

### Solution

**Step 1: Define Robot State Enum**

Add to `fms.rs`:

```rust
/// Valid robot states following FIRST FMS logic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotState {
    /// Robot not connected to FMS
    Disconnected,
    /// Robot connected (TCP heartbeat received)
    Connected,
    /// Robot ready to be enabled (passed pre-match checks)
    Ready,
    /// Robot enabled and running
    Enabled,
    /// E-stop triggered (disabled until manual reset)
    EStoppedGlobal,
    /// Per-robot e-stop triggered
    EStoppedRobot,
}

impl RobotState {
    /// Determine valid next states from current state
    pub fn valid_transitions(&self) -> &'static [RobotState] {
        match self {
            RobotState::Disconnected => &[RobotState::Connected],
            RobotState::Connected => &[RobotState::Ready, RobotState::Disconnected],
            RobotState::Ready => &[RobotState::Enabled, RobotState::Connected, RobotState::Disconnected],
            RobotState::Enabled => &[RobotState::Ready, RobotState::Disconnected, RobotState::EStoppedRobot],
            RobotState::EStoppedRobot => &[RobotState::Ready, RobotState::Disconnected],
            RobotState::EStoppedGlobal => &[RobotState::Ready, RobotState::Disconnected],
        }
    }

    /// Check if transition is valid
    pub fn can_transition_to(&self, next: RobotState) -> bool {
        self.valid_transitions().contains(&next)
    }
}
```

**Step 2: Add State to DriverstationConnection**

**BEFORE:**
```rust
pub struct DriverstationConnection {
    pub team_number: u16,
    pub ds_control: DSControl,
    pub alliance_station: u8,
    pub control_mode: [ControlMode; 2],
    pub estop_state: EStopState,
    pub last_heartbeat: std::time::Instant,
}
```

**AFTER:**
```rust
pub struct DriverstationConnection {
    pub team_number: u16,
    pub ds_control: DSControl,
    pub alliance_station: u8,
    pub control_mode: [ControlMode; 2],
    pub estop_state: EStopState,
    pub last_heartbeat: std::time::Instant,
    pub state: RobotState,
}

impl DriverstationConnection {
    pub fn new(team_number: u16) -> Self {
        Self {
            team_number,
            state: RobotState::Connected,
            last_heartbeat: std::time::Instant::now(),
            ..Default::default()
        }
    }

    /// Transition to new state with validation
    pub fn transition_to(&mut self, new_state: RobotState) -> anyhow::Result<()> {
        if !self.state.can_transition_to(new_state) {
            anyhow::bail!(
                "Invalid state transition for team {}: {:?} -> {:?}",
                self.team_number,
                self.state,
                new_state
            );
        }

        info!(
            team_number = self.team_number,
            from_state = ?self.state,
            to_state = ?new_state,
            "Robot state transition"
        );

        self.state = new_state;
        Ok(())
    }

    /// Check if robot can receive enable command
    pub fn can_be_enabled(&self) -> bool {
        self.state == RobotState::Ready && self.estop_state == EStopState::Active
    }
}
```

**Step 3: Enforce State in UDP Sending**

Update `new_driverstation()` to only send UDP commands when in Enabled state:

**In udp packet sending loop:**
```rust
// Before creating and sending UDP packet
if driverstation_connection.state != RobotState::Enabled {
    debug!(
        team_number,
        state = ?driverstation_connection.state,
        "Skipping UDP send - robot not enabled"
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    continue;
}

if driverstation_connection.is_estop_active() {
    debug!(team_number, "Skipping UDP send - e-stop active");
    tokio::time::sleep(Duration::from_millis(500)).await;
    continue;
}

// Now safe to send UDP
match shared_udp_socket.send_to(...).await {
    // ... send logic
}
```

---

## P2.4: Command Validation

### Solution

Add validation functions to `tcp.rs`:

```rust
/// Validate that a team number is in the allowed list
fn validate_team_number(team_number: u16, allowed_teams: &[u16]) -> anyhow::Result<()> {
    if allowed_teams.contains(&team_number) {
        Ok(())
    } else {
        anyhow::bail!(
            "Team {} is not in allowed teams list: {:?}",
            team_number,
            allowed_teams
        );
    }
}

/// Validate alliance station value
fn validate_alliance_station(alliance_station: u8) -> anyhow::Result<()> {
    match alliance_station {
        1..=3 | 101..=103 => Ok(()), // RED_1-3 or BLUE_1-3
        _ => anyhow::bail!(
            "Invalid alliance station {}: must be 1-3 (RED) or 101-103 (BLUE)",
            alliance_station
        ),
    }
}

/// Validate control mode values
fn validate_control_mode(mode: &ControlMode) -> anyhow::Result<()> {
    match mode {
        ControlMode::Teleop | ControlMode::Auto | ControlMode::Test
        | ControlMode::Enabled | ControlMode::Disabled => Ok(()),
        _ => anyhow::bail!("Unknown control mode: {:?}", mode),
    }
}
```

Use these in the TCP handler:

```rust
TagType::TeamNumber(tn) => {
    // Validate team number
    validate_team_number(tn.team_number, &fms_lock.allowed_driverstations)
        .context("Team number validation failed")?;

    // Check if already registered
    if fms_lock.driverstations.iter().any(|ds| ds.team_number == tn.team_number) {
        warn!(team_number = tn.team_number, "Team re-registering - removing old entry");
        fms_lock.driverstations.retain(|ds| ds.team_number != tn.team_number);
    }

    let mut new_ds = DriverstationConnection::new(tn.team_number);
    new_ds.alliance_station = 1; // Default to RED_1 (should come from config)

    fms_lock.driverstations.push(new_ds);

    info!(
        team_number = tn.team_number,
        alliance_station = 1,
        "Driverstation registered and validated"
    );
}
```

---

## P2.5: Graceful Degradation

### Problem
If the UDP socket fails, or FMS lock deadlocks, the entire system fails unsafely.

### Solution

**Step 1: UDP Socket Failure Handling**

In `new_driverstation()`:

```rust
pub async fn new_driverstation(...) -> anyhow::Result<()> {
    // ...
    
    loop {
        // ... get driverstation_ip
        
        match shared_udp_socket.send_to(...).await {
            Ok(bytes_sent) => {
                debug!(team_number, bytes_sent, "UDP sent");
                consecutive_send_failures = 0;
            }
            Err(e) => {
                consecutive_send_failures += 1;
                error!(
                    team_number,
                    error = ?e,
                    attempt = consecutive_send_failures,
                    "UDP send failed"
                );

                // After too many failures, disable robot and exit
                if consecutive_send_failures >= 10 {
                    error!(
                        team_number,
                        "UDP sender giving up after 10 consecutive failures"
                    );

                    // Disable the robot in FMS
                    if let Ok(mut fms_lock) = fms.lock() {
                        if let Some(ds) = fms_lock.driverstations.iter_mut()
                            .find(|d| d.team_number == team_number)
                        {
                            ds.control_mode = [ControlMode::Disabled, ControlMode::Disabled];
                            warn!(team_number, "Robot disabled due to UDP failure");
                        }
                    }

                    anyhow::bail!("UDP socket failed - giving up");
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```

**Step 2: FMS Lock Timeout Detection**

Add lock timeout wrapper:

```rust
/// Acquire FMS lock with timeout to detect deadlocks
pub async fn acquire_fms_lock_with_timeout(
    fms: Arc<Mutex<FMS>>,
    timeout_ms: u64,
) -> anyhow::Result<std::sync::MutexGuard<'static, FMS>> {
    let start = std::time::Instant::now();
    
    loop {
        if start.elapsed().as_millis() > timeout_ms as u128 {
            error!(
                timeout_ms,
                "FMS lock acquisition timeout - possible deadlock!"
            );
            anyhow::bail!(
                "FMS lock acquisition timeout after {}ms - possible deadlock",
                timeout_ms
            );
        }

        match fms.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
}
```

**Step 3: HTTP Server Isolation**

The HTTP server should be spawned such that if it fails, it doesn't crash the main server:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... TCP listener
    
    // HTTP server in separate spawned task
    let fms_for_http = fms.clone();
    tokio::spawn(async move {
        let result = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(fms_for_http.clone()))
                // ... routes
        })
        .bind(HTTP_SERVER_PORT)
        .and_then(|server| Ok(server.run()))
        .await;

        match result {
            Ok(_) => info!("HTTP server completed"),
            Err(e) => error!(error = ?e, "HTTP server failed"),
        }
    });

    // Main loop continues regardless of HTTP status
    tokio::time::sleep(Duration::from_secs(3600)).await; // Run for 1 hour
    
    Ok(())
}
```

---

## Phase 2 Completion Checklist

- [ ] Added `EStopState` enum to `DriverstationConnection`
- [ ] Added `GlobalEStopState` to `FMS`
- [ ] Implemented e-stop helper methods on both structures
- [ ] Created e-stop API endpoints (trigger, clear, status)
- [ ] Registered e-stop routes in HTTP server
- [ ] Created connection monitor background task
- [ ] Updated heartbeat on packet receive
- [ ] Spawned monitor task in main
- [ ] Defined `RobotState` enum with valid transitions
- [ ] Added state to `DriverstationConnection`
- [ ] Implemented state transition validation
- [ ] Added state checks to UDP sending logic
- [ ] Implemented command validation functions
- [ ] Added UDP failure handling with circuit breaker
- [ ] Created lock acquisition timeout wrapper
- [ ] Isolated HTTP server in separate task
- [ ] Tested e-stop endpoints manually
- [ ] Tested heartbeat monitoring (disconnect detection)
- [ ] Tested state transitions
- [ ] `cargo build` passes
- [ ] `cargo clippy` passes

---

## Summary

Phase 2 adds critical safety features:
- ✅ Emergency stop mechanisms (per-robot and global)
- ✅ Stale command prevention (disconnect detection)
- ✅ State machine enforcement (valid transitions only)
- ✅ Graceful degradation (system doesn't crash on subsystem failures)

After Phase 2, the system is safe to operate around robots and can be exposed to end users via the REST API.

