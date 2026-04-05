# Phase 4: Telemetry & Logging - Event Tracking & Historical Data

**Objective:** Implement comprehensive logging, event tracking, and telemetry persistence for debugging and team analysis.

**Duration:** ~2 hours  
**Status:** Depends on Phase 3 completion  
**Depends On:** P3.1-P3.6 (all API endpoints)

---

## Overview

Phase 4 adds observability and historical data collection. The system will:
- Log all events (connections, match events, e-stops, errors)
- Track per-robot telemetry (battery voltage, mode changes, errors)
- Persist data to SQLite database
- Provide API endpoints for historical data retrieval
- Enable debugging and team performance analysis

---

## P4.1: Event Logging System

### Problem
Currently, errors and events are only logged to console. There's no persistent record or queryable event history.

### Solution

**Step 1: Define Event Types**

Create new file `src/driverstation_comms/events.rs`:

```rust
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// All FMS system events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FmsEvent {
    // Connection events
    DriverstationConnected { team: u16 },
    DriverstationDisconnected { team: u16, reason: String },
    DriverstationTimeout { team: u16, idle_secs: u64 },
    
    // Match events
    MatchStarted { enabled_teams: Vec<u16> },
    MatchStopped { reason: String },
    MatchPaused,
    MatchResumed,
    
    // E-stop events
    EStopTriggered { team: Option<u16> }, // None = global
    EStopCleared { team: Option<u16> },
    
    // Robot state events
    RobotEnabled { team: u16 },
    RobotDisabled { team: u16, reason: String },
    RobotStateTransition { team: u16, from_state: String, to_state: String },
    
    // Error events
    ParseError { error: String, packet_bytes: Option<String> },
    LockError { error: String },
    ProtocolError { team: u16, error: String },
    
    // System events
    SystemStarted,
    SystemShutdown { reason: String },
}

/// Persistent event record with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp_ms: i64,
    pub event_type: String,
    pub team_number: Option<u16>,
    pub event_data: serde_json::Value,
    pub severity: String, // "debug", "info", "warn", "error"
}

impl EventRecord {
    pub fn new(event: FmsEvent, severity: &str) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let (event_type, team_number, event_data) = match event {
            FmsEvent::DriverstationConnected { team } => (
                "DriverstationConnected".to_string(),
                Some(team),
                serde_json::json!({ "team": team }),
            ),
            FmsEvent::DriverstationDisconnected { team, reason } => (
                "DriverstationDisconnected".to_string(),
                Some(team),
                serde_json::json!({ "team": team, "reason": reason }),
            ),
            FmsEvent::MatchStarted { enabled_teams } => (
                "MatchStarted".to_string(),
                None,
                serde_json::json!({ "enabled_teams": enabled_teams, "count": enabled_teams.len() }),
            ),
            FmsEvent::EStopTriggered { team } => (
                "EStopTriggered".to_string(),
                team,
                serde_json::json!({ "is_global": team.is_none() }),
            ),
            // ... handle other event types similarly
            _ => (
                format!("{:?}", event),
                None,
                serde_json::json!(event),
            ),
        };

        Self {
            timestamp_ms,
            event_type,
            team_number,
            event_data,
            severity: severity.to_string(),
        }
    }
}
```

**Step 2: Create Event Logger with Circular Buffer**

Add to `src/driverstation_comms/mod.rs`:

```rust
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

const MAX_EVENTS_IN_MEMORY: usize = 10_000;

/// In-memory circular buffer for recent events
pub struct EventLogger {
    events: VecDeque<EventRecord>,
}

impl EventLogger {
    pub fn new() -> Self {
        Self {
            events: VecDeque::with_capacity(MAX_EVENTS_IN_MEMORY),
        }
    }

    /// Log a new event
    pub fn log(&mut self, event: FmsEvent, severity: &str) {
        let record = EventRecord::new(event, severity);
        
        // Maintain circular buffer - remove oldest if at capacity
        if self.events.len() >= MAX_EVENTS_IN_MEMORY {
            self.events.pop_front();
        }
        
        self.events.push_back(record);
    }

    /// Get recent events (newest first)
    pub fn get_recent(&self, limit: usize) -> Vec<EventRecord> {
        self.events
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Filter events by team
    pub fn get_team_events(&self, team: u16, limit: usize) -> Vec<EventRecord> {
        self.events
            .iter()
            .filter(|e| e.team_number == Some(team))
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Filter events by type
    pub fn get_events_by_type(&self, event_type: &str, limit: usize) -> Vec<EventRecord> {
        self.events
            .iter()
            .filter(|e| e.event_type == event_type)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get total event count
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

impl Default for EventLogger {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Add EventLogger to FMS**

Update `fms.rs`:

```rust
pub struct FMS {
    pub driverstations: Vec<DriverstationConnection>,
    pub allowed_driverstations: Vec<u16>,
    pub global_estop: GlobalEStopState,
    pub global_estop_triggered_at: Option<std::time::Instant>,
    pub event_logger: Arc<Mutex<EventLogger>>,  // ← Add this
}

impl Default for FMS {
    fn default() -> Self {
        Self {
            driverstations: Vec::new(),
            allowed_driverstations: Vec::new(),
            global_estop: GlobalEStopState::Normal,
            global_estop_triggered_at: None,
            event_logger: Arc::new(Mutex::new(EventLogger::new())),
        }
    }
}

// Helper method for logging
impl FMS {
    pub fn log_event(&self, event: FmsEvent, severity: &str) {
        if let Ok(mut logger) = self.event_logger.lock() {
            logger.log(event, severity);
        }
    }
}
```

**Step 4: Log Events Throughout Codebase**

In `tcp.rs`:

```rust
// When driverstation connects
TagType::TeamNumber(tn) => {
    let mut fms_lock = fms.lock()
        .map_err(|e| anyhow::anyhow!("FMS lock poisoned: {}", e))?;

    let mut new_ds = DriverstationConnection::new(tn.team_number);
    fms_lock.driverstations.push(new_ds);

    // Log event
    fms_lock.log_event(
        FmsEvent::DriverstationConnected { team: tn.team_number },
        "info"
    );

    info!(team_number = tn.team_number, "Driverstation registered");
}

// When e-stop is triggered
pub fn trigger_global_estop(&mut self) {
    if self.global_estop != GlobalEStopState::TriggeredGlobal {
        self.log_event(FmsEvent::EStopTriggered { team: None }, "error");
        
        // ... rest of function
    }
}
```

In connection monitor:

```rust
// When driverstation times out
if ds.is_idle(HEARTBEAT_TIMEOUT_SECS) {
    // Log before disconnecting
    {
        let fms_lock = fms.lock().ok();
        if let Some(lock) = fms_lock {
            lock.log_event(
                FmsEvent::DriverstationTimeout {
                    team: ds.team_number,
                    idle_secs: HEARTBEAT_TIMEOUT_SECS,
                },
                "warn"
            );
        }
    }
    
    // ... disconnect logic
}
```

---

## P4.2: Per-Robot Telemetry Tracking

### Solution

**Step 1: Define Telemetry Data Structure**

Add to `events.rs`:

```rust
/// Per-robot telemetry sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub timestamp_ms: i64,
    pub team_number: u16,
    pub battery_voltage: Option<f32>,
    pub battery_xx: u8,
    pub battery_yy: u8,
    pub control_mode: String,
    pub enabled: bool,
    pub robot_status: u8,
    pub can_status: u8,
    pub signal_db: u8,
    pub lost_packets: u8,
    pub bandwidth: u16,
}

impl TelemetrySample {
    pub fn new(team: u16) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            timestamp_ms,
            team_number: team,
            battery_voltage: None,
            battery_xx: 0,
            battery_yy: 0,
            control_mode: "Unknown".to_string(),
            enabled: false,
            robot_status: 0,
            can_status: 0,
            signal_db: 0,
            lost_packets: 0,
            bandwidth: 0,
        }
    }
}
```

**Step 2: Add Telemetry to DriverstationConnection**

```rust
pub struct DriverstationConnection {
    // ... existing fields
    pub last_telemetry_sample: Option<TelemetrySample>,
    pub telemetry_history: VecDeque<TelemetrySample>, // Keep last 100 samples
}

impl DriverstationConnection {
    pub fn update_telemetry(&mut self, sample: TelemetrySample) {
        // Keep last 100 samples
        if self.telemetry_history.len() >= 100 {
            self.telemetry_history.pop_front();
        }
        self.telemetry_history.push_back(sample.clone());
        self.last_telemetry_sample = Some(sample);
    }
}
```

**Step 3: Populate Telemetry from LogData Packets**

In `tcp.rs`:

```rust
0x16 => {
    debug!("Parsing LogData packet");
    
    let trip_time = safe_pop(&mut data)?;
    let lost_packets = safe_pop(&mut data)?;
    let battery_xx = safe_pop(&mut data)? as u16;
    let battery_yy = safe_pop(&mut data)? as u16;
    let robot_status = safe_pop(&mut data)?;
    let can = safe_pop(&mut data)?;
    let signal_db = safe_pop(&mut data)?;
    let bandwidth = safe_pop_u16(&mut data)?;
    
    let battery = (battery_xx + battery_yy) / 256;
    
    tag_type = TagType::LogData(LogData {
        trip_time,
        lost_packets,
        battery,
        robot_status,
        can,
        signal_db,
        bandwidth,
    });
    
    // Create telemetry sample from this data
    // Note: We don't know which team this is from yet - will update after registration
}
```

Then in the main handler, after matching on TeamNumber:

```rust
TagType::LogData(log_data) => {
    // Update telemetry for currently-connected driverstations
    let mut fms_lock = fms.lock()?;
    
    for ds in fms_lock.driverstations.iter_mut() {
        let mut sample = TelemetrySample::new(ds.team_number);
        sample.battery_xx = log_data.battery_xx as u8;
        sample.battery_yy = log_data.battery_yy as u8;
        sample.battery_voltage = Some(log_data.battery as f32 / 256.0);
        sample.control_mode = format!("{:?}", ds.control_mode[0]);
        sample.enabled = ds.control_mode[1] == ControlMode::Enabled;
        sample.robot_status = log_data.robot_status;
        sample.can_status = log_data.can;
        sample.signal_db = log_data.signal_db;
        sample.lost_packets = log_data.lost_packets;
        sample.bandwidth = log_data.bandwidth;
        
        ds.update_telemetry(sample);
    }
}
```

---

## P4.3: SQLite Persistence

### Solution

**Step 1: Add SQLite Dependency**

Update `Cargo.toml`:

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled", "chrono"] }
```

**Step 2: Create Database Module**

Create `src/db.rs`:

```rust
use rusqlite::{Connection, Result as SqlResult, params};
use crate::driverstation_comms::events::{EventRecord, TelemetrySample};
use std::sync::{Arc, Mutex};
use tracing::{info, error};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        
        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY,
                timestamp_ms INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                team_number INTEGER,
                event_data TEXT NOT NULL,
                severity TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE TABLE IF NOT EXISTS telemetry (
                id INTEGER PRIMARY KEY,
                timestamp_ms INTEGER NOT NULL,
                team_number INTEGER NOT NULL,
                battery_voltage REAL,
                battery_xx INTEGER,
                battery_yy INTEGER,
                control_mode TEXT,
                enabled INTEGER,
                robot_status INTEGER,
                can_status INTEGER,
                signal_db INTEGER,
                lost_packets INTEGER,
                bandwidth INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_ms);
            CREATE INDEX IF NOT EXISTS idx_events_team ON events(team_number);
            CREATE INDEX IF NOT EXISTS idx_telemetry_team ON telemetry(team_number);
            CREATE INDEX IF NOT EXISTS idx_telemetry_timestamp ON telemetry(timestamp_ms);"
        )?;

        info!("Database initialized at {}", path);
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Store an event
    pub fn store_event(&self, event: &EventRecord) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "INSERT INTO events (timestamp_ms, event_type, team_number, event_data, severity)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.timestamp_ms,
                &event.event_type,
                event.team_number,
                event.event_data.to_string(),
                &event.severity,
            ],
        )?;

        Ok(())
    }

    /// Store telemetry sample
    pub fn store_telemetry(&self, sample: &TelemetrySample) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "INSERT INTO telemetry (
                timestamp_ms, team_number, battery_voltage, battery_xx, battery_yy,
                control_mode, enabled, robot_status, can_status, signal_db,
                lost_packets, bandwidth
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                sample.timestamp_ms,
                sample.team_number,
                sample.battery_voltage,
                sample.battery_xx,
                sample.battery_yy,
                &sample.control_mode,
                sample.enabled as i32,
                sample.robot_status,
                sample.can_status,
                sample.signal_db,
                sample.lost_packets,
                sample.bandwidth,
            ],
        )?;

        Ok(())
    }

    /// Get events in time range
    pub fn get_events(
        &self,
        start_ms: i64,
        end_ms: i64,
        team: Option<u16>,
    ) -> SqlResult<Vec<EventRecord>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = if let Some(t) = team {
            conn.prepare(
                "SELECT timestamp_ms, event_type, team_number, event_data, severity
                 FROM events
                 WHERE timestamp_ms BETWEEN ?1 AND ?2 AND team_number = ?3
                 ORDER BY timestamp_ms DESC"
            )?
        } else {
            conn.prepare(
                "SELECT timestamp_ms, event_type, team_number, event_data, severity
                 FROM events
                 WHERE timestamp_ms BETWEEN ?1 AND ?2
                 ORDER BY timestamp_ms DESC"
            )?
        };

        let events = if let Some(t) = team {
            stmt.query_map(params![start_ms, end_ms, t], |row| {
                Ok(EventRecord {
                    timestamp_ms: row.get(0)?,
                    event_type: row.get(1)?,
                    team_number: row.get(2)?,
                    event_data: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(serde_json::json!({})),
                    severity: row.get(4)?,
                })
            })?
        } else {
            stmt.query_map(params![start_ms, end_ms], |row| {
                Ok(EventRecord {
                    timestamp_ms: row.get(0)?,
                    event_type: row.get(1)?,
                    team_number: row.get(2)?,
                    event_data: serde_json::from_str(&row.get::<_, String>(3)?)
                        .unwrap_or(serde_json::json!({})),
                    severity: row.get(4)?,
                })
            })?
        }
        .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Get telemetry in time range
    pub fn get_telemetry(
        &self,
        team: u16,
        start_ms: i64,
        end_ms: i64,
    ) -> SqlResult<Vec<TelemetrySample>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT timestamp_ms, team_number, battery_voltage, battery_xx, battery_yy,
                    control_mode, enabled, robot_status, can_status, signal_db,
                    lost_packets, bandwidth
             FROM telemetry
             WHERE team_number = ?1 AND timestamp_ms BETWEEN ?2 AND ?3
             ORDER BY timestamp_ms DESC"
        )?;

        let samples = stmt.query_map(params![team, start_ms, end_ms], |row| {
            Ok(TelemetrySample {
                timestamp_ms: row.get(0)?,
                team_number: row.get(1)?,
                battery_voltage: row.get(2)?,
                battery_xx: row.get(3)?,
                battery_yy: row.get(4)?,
                control_mode: row.get(5)?,
                enabled: row.get::<_, i32>(6)? != 0,
                robot_status: row.get(7)?,
                can_status: row.get(8)?,
                signal_db: row.get(9)?,
                lost_packets: row.get(10)?,
                bandwidth: row.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(samples)
    }
}
```

**Step 3: Integrate Database with FMS**

Update `main.rs`:

```rust
mod db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... initialization
    
    // Initialize database
    let database = db::Database::new("fms_telemetry.db")
        .context("Failed to initialize database")?;
    let db_arc = Arc::new(database);
    
    // Periodically flush events and telemetry to database
    let db_for_flush = db_arc.clone();
    let fms_for_flush = fms.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await; // Flush every 10 seconds
            
            if let Ok(fms_lock) = fms_for_flush.lock() {
                // Flush events
                if let Ok(logger) = fms_lock.event_logger.lock() {
                    // Get recent events that haven't been flushed yet
                    // (Would need to track last flush timestamp)
                }
                
                // Flush telemetry
                for ds in fms_lock.driverstations.iter() {
                    if let Some(sample) = &ds.last_telemetry_sample {
                        if let Err(e) = db_for_flush.store_telemetry(sample) {
                            error!(error = ?e, "Failed to store telemetry");
                        }
                    }
                }
            }
        }
    });
}
```

---

## P4.4: Telemetry API Endpoints

### Solution

**Add to interface module in main.rs:**

```rust
/// GET /api/telemetry/history - Retrieve historical telemetry
pub async fn get_telemetry_history(
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, ApiError> {
    let team = query
        .get("team")
        .and_then(|t| t.parse::<u16>().ok())
        .ok_or_else(|| ApiError {
            code: StatusCode::BAD_REQUEST,
            message: "Missing or invalid 'team' query parameter".to_string(),
        })?;

    let start_ms = query
        .get("start_time")
        .and_then(|t| t.parse::<i64>().ok())
        .unwrap_or_else(|| {
            // Default: last 24 hours
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
                - (24 * 60 * 60 * 1000)
        });

    let end_ms = query
        .get("end_time")
        .and_then(|t| t.parse::<i64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        });

    let format = query.get("format").map(|s| s.as_str()).unwrap_or("json");

    match db.get_telemetry(team, start_ms, end_ms) {
        Ok(samples) => {
            match format {
                "csv" => {
                    let csv_content = generate_csv(&samples);
                    Ok(HttpResponse::Ok()
                        .content_type("text/csv")
                        .body(csv_content))
                }
                "json" => {
                    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
                        "team": team,
                        "start_ms": start_ms,
                        "end_ms": end_ms,
                        "sample_count": samples.len(),
                        "samples": samples
                    }))))
                }
                _ => {
                    Err(ApiError {
                        code: StatusCode::BAD_REQUEST,
                        message: format!("Unknown format: {}", format),
                    })
                }
            }
        }
        Err(e) => {
            Err(ApiError {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Database query failed: {}", e),
            })
        }
    }
}

/// GET /api/events/history - Retrieve historical events
pub async fn get_event_history(
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Arc<Database>>,
) -> Result<HttpResponse, ApiError> {
    let start_ms = query
        .get("start_time")
        .and_then(|t| t.parse::<i64>().ok())
        .unwrap_or_else(|| {
            // Default: last 24 hours
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
                - (24 * 60 * 60 * 1000)
        });

    let end_ms = query
        .get("end_time")
        .and_then(|t| t.parse::<i64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64
        });

    let team = query.get("team").and_then(|t| t.parse::<u16>().ok());

    match db.get_events(start_ms, end_ms, team) {
        Ok(events) => {
            Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
                "start_ms": start_ms,
                "end_ms": end_ms,
                "team_filter": team,
                "event_count": events.len(),
                "events": events
            }))))
        }
        Err(e) => {
            Err(ApiError {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Database query failed: {}", e),
            })
        }
    }
}

/// Helper: Convert telemetry to CSV
fn generate_csv(samples: &[TelemetrySample]) -> String {
    let mut csv = String::from(
        "timestamp_ms,team,battery_voltage,control_mode,enabled,robot_status,can_status,signal_db,lost_packets,bandwidth\n"
    );

    for sample in samples {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            sample.timestamp_ms,
            sample.team_number,
            sample.battery_voltage.unwrap_or(0.0),
            sample.control_mode,
            if sample.enabled { 1 } else { 0 },
            sample.robot_status,
            sample.can_status,
            sample.signal_db,
            sample.lost_packets,
            sample.bandwidth,
        ));
    }

    csv
}
```

---

## P4.5: Structured Logging Infrastructure

### Already Partially Complete

From Phase 1, tracing is initialized. Now enhance it:

**In main.rs:**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Enhanced tracing setup
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        // Write to file in production
        .with_writer(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("fms.log")
                .unwrap_or_else(|_| std::fs::File::create("fms.log").unwrap())
        )
        .init();

    info!("🚀 FMS Server Starting");
}
```

**Add log rotation (create `src/logging.rs`):**

```rust
use std::fs;
use std::path::Path;
use tracing::info;

const MAX_LOG_SIZE_MB: u64 = 100;
const MAX_LOG_FILES: usize = 5;

pub fn setup_log_rotation(log_dir: &str) {
    let path = Path::new(log_dir);
    
    if !path.exists() {
        fs::create_dir_all(path).ok();
    }

    // Check if current log is too large
    let current_log = path.join("fms.log");
    if current_log.exists() {
        if let Ok(metadata) = fs::metadata(&current_log) {
            let size_mb = metadata.len() / (1024 * 1024);
            if size_mb > MAX_LOG_SIZE_MB {
                rotate_logs(path);
            }
        }
    }
}

fn rotate_logs(log_dir: &Path) {
    // fms.log -> fms.log.1, fms.log.1 -> fms.log.2, etc.
    for i in (1..MAX_LOG_FILES).rev() {
        let from = log_dir.join(format!("fms.log.{}", i));
        let to = log_dir.join(format!("fms.log.{}", i + 1));
        fs::rename(&from, &to).ok();
    }

    // Rename current to fms.log.1
    let current = log_dir.join("fms.log");
    let rotated = log_dir.join("fms.log.1");
    fs::rename(&current, &rotated).ok();

    info!("Log files rotated");
}
```

---

## P4.6: Register Telemetry Routes

Update HTTP server:

```rust
let _ = HttpServer::new(move || {
    App::new()
        // ... existing routes
        
        // Telemetry endpoints
        .route("/api/telemetry/live", web::get().to(interface::get_live_telemetry))
        .route("/api/telemetry/history", web::get().to(interface::get_telemetry_history))
        .route("/api/events/history", web::get().to(interface::get_event_history))
})
```

---

## API Examples

### Get team telemetry history
```bash
curl "http://localhost:2000/api/telemetry/history?team=1234&start_time=1704067200000&end_time=1704153600000"
```

Response:
```json
{
  "success": true,
  "data": {
    "team": 1234,
    "start_ms": 1704067200000,
    "end_ms": 1704153600000,
    "sample_count": 42,
    "samples": [
      {
        "timestamp_ms": 1704153599000,
        "team_number": 1234,
        "battery_voltage": 11.8,
        "control_mode": "Teleop",
        "enabled": true,
        ...
      }
    ]
  }
}
```

### Export as CSV
```bash
curl "http://localhost:2000/api/telemetry/history?team=1234&format=csv" > telemetry.csv
```

### Get event history
```bash
curl "http://localhost:2000/api/events/history?start_time=1704067200000&team=1234"
```

---

## Phase 4 Completion Checklist

- [ ] Created `FmsEvent` enum in `events.rs`
- [ ] Created `EventRecord` struct
- [ ] Created `EventLogger` with circular buffer
- [ ] Added `event_logger` to `FMS`
- [ ] Added logging calls throughout codebase
- [ ] Created `TelemetrySample` struct
- [ ] Added telemetry tracking to `DriverstationConnection`
- [ ] Updated LogData handler to populate telemetry
- [ ] Created `Database` module with SQLite
- [ ] Implemented event and telemetry storage
- [ ] Created telemetry history API endpoint
- [ ] Created event history API endpoint
- [ ] Implemented CSV export
- [ ] Enhanced logging with file output
- [ ] Implemented log rotation
- [ ] Registered all telemetry routes
- [ ] Tested database queries
- [ ] Tested telemetry API responses
- [ ] `cargo build` passes
- [ ] `cargo clippy` passes

---

## Summary

Phase 4 provides comprehensive observability:
- ✅ Persistent event logging
- ✅ Per-robot telemetry collection
- ✅ Historical data querying
- ✅ CSV export for analysis
- ✅ Structured logging infrastructure

After Phase 4, teams can analyze robot performance and troubleshoot issues using historical data.

