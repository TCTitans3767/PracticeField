# Phase 3: Core API Features - REST Endpoints for Frontend

**Objective:** Create REST API endpoints for frontend integration: robot management, match control, and real-time monitoring.

**Duration:** ~2-2.5 hours  
**Status:** Depends on Phase 2 completion  
**Depends On:** P2.1-P2.5 (all safety features)

---

## Overview

Phase 3 exposes the FMS functionality to a frontend application via REST APIs. The frontend can:
- List connected robots and their status
- Register/deregister teams
- Start/stop/pause/resume matches
- Trigger e-stops
- Monitor live robot status

All endpoints return JSON and use consistent error handling.

---

## P3.1: REST API Framework Setup

### Solution

**Step 1: Update Cargo.toml Dependencies**

Ensure your `Cargo.toml` has:

```toml
[dependencies]
actix-web = "4.13.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.50.0", features = ["full", "rt"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0.102"

[dev-dependencies]
tokio-test = "0.4"
```

**Step 2: Create Consistent Response Types**

In `main.rs`, create response wrapper types:

```rust
use serde::{Deserialize, Serialize};
use std::fmt;

/// Consistent API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Local::now().to_rfc3339(),
        }
    }

    pub fn error(msg: &str) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(msg.to_string()),
            timestamp: chrono::Local::now().to_rfc3339(),
        }
    }
}

/// Request/response for match control
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatchControlRequest {
    pub action: String, // "start", "stop", "pause", "resume"
}

/// Robot status for API responses
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RobotStatusResponse {
    pub team_number: u16,
    pub connected: bool,
    pub enabled: bool,
    pub battery_voltage: Option<f32>,
    pub control_mode: String,
    pub estop_active: bool,
    pub state: String,
    pub last_heartbeat_ms_ago: u64,
}

/// Match status response
#[derive(Debug, Serialize, Deserialize)]
pub struct MatchStatusResponse {
    pub match_active: bool,
    pub global_estop: String,
    pub enabled_teams: Vec<u16>,
    pub total_teams: usize,
    pub robots: Vec<RobotStatusResponse>,
}
```

**Step 3: Create API Error Handler**

```rust
use actix_web::{error::ResponseError, http::StatusCode};

#[derive(Debug)]
pub struct ApiError {
    pub code: StatusCode,
    pub message: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.code
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponse::build(self.status_code()).json(json!({
            "success": false,
            "error": self.message,
            "timestamp": chrono::Local::now().to_rfc3339()
        }))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}
```

**Step 4: Request Logging Middleware**

```rust
use actix_web::middleware::Logger;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use futures::future::LocalBoxFuture;

/// Middleware to log all API requests
pub async fn setup_logging_middleware(app: actix_web::App) -> actix_web::App {
    app.wrap(Logger::default())
}

// Or use tracing directly:
pub fn log_request(req: &actix_web::HttpRequest) {
    info!(
        method = %req.method(),
        path = req.path(),
        "API request received"
    );
}
```

---

## P3.2: Robot Connection Management

### Solution

**Add to `main.rs` interface module:**

```rust
/// GET /api/robots - List all connected robots
pub async fn list_robots(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Listing robots");
    
    let fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    let robots: Vec<RobotStatusResponse> = fms_lock.driverstations
        .iter()
        .map(|ds| RobotStatusResponse {
            team_number: ds.team_number,
            connected: true,
            enabled: ds.control_mode[1] == ControlMode::Enabled,
            battery_voltage: None, // Will be populated from LogData
            control_mode: format!("{:?}", ds.control_mode[0]),
            estop_active: ds.is_estop_active(),
            state: format!("{:?}", ds.state),
            last_heartbeat_ms_ago: ds.last_heartbeat.elapsed().as_millis() as u64,
        })
        .collect();

    info!(robot_count = robots.len(), "Robots listed successfully");

    Ok(HttpResponse::Ok().json(ApiResponse::ok(robots)))
}

/// POST /api/robots/{team}/register - Register a team
pub async fn register_robot(
    team_id: web::Path<u16>,
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let team_number = team_id.into_inner();
    info!(team_number, "API: Registering robot");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // Check if already registered
    if fms_lock.driverstations.iter().any(|ds| ds.team_number == team_number) {
        return Err(ApiError {
            code: StatusCode::CONFLICT,
            message: format!("Team {} already registered", team_number),
        });
    }

    // Add to allowed teams list
    fms_lock.allowed_driverstations.push(team_number);

    info!(team_number, "Robot registered - awaiting TCP connection");

    Ok(HttpResponse::Created().json(ApiResponse::ok(json!({
        "team": team_number,
        "status": "registered",
        "message": "Team registered successfully. Awaiting TCP connection from driverstation."
    }))))
}

/// POST /api/robots/{team}/deregister - Deregister a team
pub async fn deregister_robot(
    team_id: web::Path<u16>,
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let team_number = team_id.into_inner();
    info!(team_number, "API: Deregistering robot");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // Remove from allowed teams
    fms_lock.allowed_driverstations.retain(|&t| t != team_number);

    // Disconnect if currently connected
    let was_connected = fms_lock.driverstations.len();
    fms_lock.driverstations.retain(|ds| ds.team_number != team_number);
    let is_connected_now = fms_lock.driverstations.len() < was_connected;

    warn!(team_number, was_connected = is_connected_now, "Robot deregistered");

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "team": team_number,
        "status": "deregistered",
        "was_connected": is_connected_now
    }))))
}

/// GET /api/robots/{team}/status - Get individual robot status
pub async fn get_robot_status(
    team_id: web::Path<u16>,
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let team_number = team_id.into_inner();

    let fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    let robot = fms_lock.driverstations
        .iter()
        .find(|ds| ds.team_number == team_number)
        .ok_or_else(|| ApiError {
            code: StatusCode::NOT_FOUND,
            message: format!("Team {} not found", team_number),
        })?;

    let status = RobotStatusResponse {
        team_number: robot.team_number,
        connected: true,
        enabled: robot.control_mode[1] == ControlMode::Enabled,
        battery_voltage: None,
        control_mode: format!("{:?}", robot.control_mode[0]),
        estop_active: robot.is_estop_active(),
        state: format!("{:?}", robot.state),
        last_heartbeat_ms_ago: robot.last_heartbeat.elapsed().as_millis() as u64,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::ok(status)))
}

/// DELETE /api/robots/{team} - Force disconnect a driverstation
pub async fn disconnect_robot(
    team_id: web::Path<u16>,
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let team_number = team_id.into_inner();
    warn!(team_number, "API: Force disconnecting robot");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    let found = fms_lock.driverstations
        .iter()
        .position(|ds| ds.team_number == team_number);

    match found {
        Some(pos) => {
            fms_lock.driverstations.remove(pos);
            info!(team_number, "Robot disconnected via API");

            Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
                "team": team_number,
                "status": "disconnected"
            }))))
        }
        None => {
            Err(ApiError {
                code: StatusCode::NOT_FOUND,
                message: format!("Team {} not connected", team_number),
            })
        }
    }
}
```

---

## P3.3: Match Control

### Solution

**Add to interface module:**

```rust
/// POST /api/match/start - Enable all registered robots
pub async fn start_match(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Starting match");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // Verify global e-stop is not active
    if fms_lock.global_estop != GlobalEStopState::Normal {
        return Err(ApiError {
            code: StatusCode::CONFLICT,
            message: "Cannot start match - global e-stop is active".to_string(),
        });
    }

    let enabled_count = fms_lock.driverstations.iter()
        .filter(|ds| {
            // Enable robots that are ready and not e-stopped
            ds.can_be_enabled()
        })
        .count();

    if enabled_count == 0 {
        return Err(ApiError {
            code: StatusCode::PRECONDITION_FAILED,
            message: "No robots ready to be enabled".to_string(),
        });
    }

    // Enable all eligible robots
    for ds in fms_lock.driverstations.iter_mut() {
        if ds.can_be_enabled() {
            ds.control_mode = [ControlMode::Teleop, ControlMode::Enabled];
            ds.state = RobotState::Enabled;
            info!(team_number = ds.team_number, "Robot enabled");
        }
    }

    let enabled_teams: Vec<u16> = fms_lock.driverstations
        .iter()
        .filter(|ds| ds.control_mode[1] == ControlMode::Enabled)
        .map(|ds| ds.team_number)
        .collect();

    info!(enabled_count = enabled_teams.len(), "Match started");

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "status": "match_started",
        "enabled_robots": enabled_teams,
        "count": enabled_teams.len()
    }))))
}

/// POST /api/match/stop - Disable all robots
pub async fn stop_match(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Stopping match");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // Disable all robots
    for ds in fms_lock.driverstations.iter_mut() {
        if ds.control_mode[1] == ControlMode::Enabled {
            ds.control_mode = [ControlMode::Teleop, ControlMode::Disabled];
            ds.state = RobotState::Ready;
            info!(team_number = ds.team_number, "Robot disabled");
        }
    }

    warn!("Match stopped - all robots disabled");

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "status": "match_stopped",
        "all_robots_disabled": true
    }))))
}

/// POST /api/match/pause - Disable robots but keep connection
pub async fn pause_match(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Pausing match");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    let paused_count = fms_lock.driverstations.iter_mut()
        .filter(|ds| {
            if ds.control_mode[1] == ControlMode::Enabled {
                ds.control_mode = [ControlMode::Teleop, ControlMode::Disabled];
                true
            } else {
                false
            }
        })
        .count();

    info!(paused_robots = paused_count, "Match paused");

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "status": "match_paused",
        "robots_paused": paused_count
    }))))
}

/// POST /api/match/resume - Re-enable paused robots
pub async fn resume_match(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Resuming match");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // Check global e-stop
    if fms_lock.global_estop != GlobalEStopState::Normal {
        return Err(ApiError {
            code: StatusCode::CONFLICT,
            message: "Cannot resume - global e-stop is active".to_string(),
        });
    }

    let resumed_count = fms_lock.driverstations.iter_mut()
        .filter(|ds| {
            if ds.can_be_enabled() && ds.control_mode[1] == ControlMode::Disabled {
                ds.control_mode = [ControlMode::Teleop, ControlMode::Enabled];
                ds.state = RobotState::Enabled;
                true
            } else {
                false
            }
        })
        .count();

    info!(resumed_robots = resumed_count, "Match resumed");

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "status": "match_resumed",
        "robots_resumed": resumed_count
    }))))
}

/// GET /api/match/status - Get current match status
pub async fn get_match_status(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    let robots = fms_lock.driverstations
        .iter()
        .map(|ds| RobotStatusResponse {
            team_number: ds.team_number,
            connected: true,
            enabled: ds.control_mode[1] == ControlMode::Enabled,
            battery_voltage: None,
            control_mode: format!("{:?}", ds.control_mode[0]),
            estop_active: ds.is_estop_active(),
            state: format!("{:?}", ds.state),
            last_heartbeat_ms_ago: ds.last_heartbeat.elapsed().as_millis() as u64,
        })
        .collect();

    let match_status = MatchStatusResponse {
        match_active: fms_lock.driverstations.iter()
            .any(|ds| ds.control_mode[1] == ControlMode::Enabled),
        global_estop: format!("{:?}", fms_lock.global_estop),
        enabled_teams: fms_lock.driverstations
            .iter()
            .filter(|ds| ds.control_mode[1] == ControlMode::Enabled)
            .map(|ds| ds.team_number)
            .collect(),
        total_teams: fms_lock.driverstations.len(),
        robots,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::ok(match_status)))
}
```

---

## P3.4: E-Stop API (Already in Phase 2)

See Phase 2 for e-stop endpoints:
- `POST /api/robots/{team}/estop` - Trigger per-robot e-stop
- `POST /api/robots/{team}/estop/clear` - Clear per-robot e-stop
- `POST /api/estop` - Trigger global e-stop
- `POST /api/estop/clear` - Clear global e-stop
- `GET /api/estop/status` - Get e-stop status

---

## P3.5: Real-Time Monitoring

### Problem
Frontend needs live updates of robot status (battery voltage, connection status, enabled/disabled state).

### Solution

**Add polling endpoint - GET /api/telemetry/live:**

```rust
/// GET /api/telemetry/live - Real-time robot telemetry
pub async fn get_live_telemetry(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    let telemetry: Vec<serde_json::Value> = fms_lock.driverstations
        .iter()
        .map(|ds| {
            json!({
                "team": ds.team_number,
                "timestamp_ms": chrono::Local::now().timestamp_millis(),
                "connected": true,
                "enabled": ds.control_mode[1] == ControlMode::Enabled,
                "control_mode": format!("{:?}", ds.control_mode[0]),
                "estop_active": ds.is_estop_active(),
                "state": format!("{:?}", ds.state),
                "alliance_station": ds.alliance_station,
                "last_heartbeat_ms_ago": ds.last_heartbeat.elapsed().as_millis() as u64,
                "ds_control": format!("{:?}", ds.ds_control),
            })
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "timestamp": chrono::Local::now().to_rfc3339(),
        "robot_count": telemetry.len(),
        "telemetry": telemetry
    }))))
}

/// Alternative: WebSocket for push updates (future enhancement)
/// This would allow real-time updates instead of polling
pub async fn websocket_telemetry(
    req: web::HttpRequest,
    stream: web::Payload,
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<impl Responder, ApiError> {
    // WebSocket implementation would go here
    // For now, document as future enhancement
    Err(ApiError {
        code: StatusCode::NOT_IMPLEMENTED,
        message: "WebSocket telemetry not yet implemented".to_string(),
    })
}
```

---

## P3.6: Register All Routes

**Update main.rs HTTP server setup:**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... initialization
    
    let fms_for_http = fms.clone();

    let _ = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(fms_for_http.clone()))
            // Health check
            .route("/status", web::get().to(interface::status))
            
            // Robot management endpoints
            .route("/api/robots", web::get().to(interface::list_robots))
            .route("/api/robots/{team}/register", web::post().to(interface::register_robot))
            .route("/api/robots/{team}/deregister", web::post().to(interface::deregister_robot))
            .route("/api/robots/{team}/status", web::get().to(interface::get_robot_status))
            .route("/api/robots/{team}", web::delete().to(interface::disconnect_robot))
            
            // Match control endpoints
            .route("/api/match/start", web::post().to(interface::start_match))
            .route("/api/match/stop", web::post().to(interface::stop_match))
            .route("/api/match/pause", web::post().to(interface::pause_match))
            .route("/api/match/resume", web::post().to(interface::resume_match))
            .route("/api/match/status", web::get().to(interface::get_match_status))
            
            // E-stop endpoints
            .route("/api/robots/{team}/estop", web::post().to(interface::trigger_robot_estop))
            .route("/api/robots/{team}/estop/clear", web::post().to(interface::clear_robot_estop))
            .route("/api/estop", web::post().to(interface::trigger_global_estop))
            .route("/api/estop/clear", web::post().to(interface::clear_global_estop))
            .route("/api/estop/status", web::get().to(interface::get_estop_status))
            
            // Telemetry endpoints
            .route("/api/telemetry/live", web::get().to(interface::get_live_telemetry))
    })
    .bind(HTTP_SERVER_PORT)
    .context("Failed to bind HTTP server")?
    .run()
    .await;

    Ok(())
}
```

---

## API Documentation

### Robot Management
```
GET    /api/robots                          - List all robots
POST   /api/robots/{team}/register          - Register team
POST   /api/robots/{team}/deregister        - Unregister team
GET    /api/robots/{team}/status            - Get robot status
DELETE /api/robots/{team}                   - Force disconnect
```

### Match Control
```
POST   /api/match/start                     - Enable robots
POST   /api/match/stop                      - Disable robots
POST   /api/match/pause                     - Pause robots
POST   /api/match/resume                    - Resume robots
GET    /api/match/status                    - Get match status
```

### E-Stop
```
POST   /api/robots/{team}/estop             - E-stop robot
POST   /api/robots/{team}/estop/clear       - Clear robot e-stop
POST   /api/estop                           - Global e-stop
POST   /api/estop/clear                     - Clear global e-stop
GET    /api/estop/status                    - E-stop status
```

### Telemetry
```
GET    /api/telemetry/live                  - Live robot telemetry
GET    /api/telemetry/history               - Historical data (Phase 4)
```

---

## Example API Requests/Responses

### Register a team
```bash
curl -X POST http://localhost:2000/api/robots/1234/register
```

Response:
```json
{
  "success": true,
  "data": {
    "team": 1234,
    "status": "registered",
    "message": "Team registered successfully. Awaiting TCP connection from driverstation."
  },
  "timestamp": "2026-04-03T02:30:00+00:00"
}
```

### Start match
```bash
curl -X POST http://localhost:2000/api/match/start
```

Response:
```json
{
  "success": true,
  "data": {
    "status": "match_started",
    "enabled_robots": [1234, 5678],
    "count": 2
  },
  "timestamp": "2026-04-03T02:30:05+00:00"
}
```

### Get live telemetry
```bash
curl http://localhost:2000/api/telemetry/live
```

Response:
```json
{
  "success": true,
  "data": {
    "timestamp": "2026-04-03T02:30:10+00:00",
    "robot_count": 2,
    "telemetry": [
      {
        "team": 1234,
        "timestamp_ms": 1748903410000,
        "connected": true,
        "enabled": true,
        "control_mode": "Teleop",
        "estop_active": false,
        "state": "Enabled",
        "last_heartbeat_ms_ago": 45
      },
      {
        "team": 5678,
        "timestamp_ms": 1748903410000,
        "connected": true,
        "enabled": true,
        "control_mode": "Teleop",
        "estop_active": false,
        "state": "Enabled",
        "last_heartbeat_ms_ago": 32
      }
    ]
  },
  "timestamp": "2026-04-03T02:30:10+00:00"
}
```

### E-stop robot
```bash
curl -X POST http://localhost:2000/api/robots/1234/estop
```

Response:
```json
{
  "success": true,
  "data": {
    "message": "E-stop triggered for team 1234",
    "team": 1234,
    "estop_state": "triggered"
  },
  "timestamp": "2026-04-03T02:30:15+00:00"
}
```

---

## Phase 3 Completion Checklist

- [ ] Added `ApiResponse<T>` wrapper type
- [ ] Added `RobotStatusResponse` and `MatchStatusResponse` types
- [ ] Created `ApiError` with proper HTTP status codes
- [ ] Implemented `list_robots()` endpoint
- [ ] Implemented `register_robot()` endpoint
- [ ] Implemented `deregister_robot()` endpoint
- [ ] Implemented `get_robot_status()` endpoint
- [ ] Implemented `disconnect_robot()` endpoint
- [ ] Implemented `start_match()` endpoint
- [ ] Implemented `stop_match()` endpoint
- [ ] Implemented `pause_match()` endpoint
- [ ] Implemented `resume_match()` endpoint
- [ ] Implemented `get_match_status()` endpoint
- [ ] Implemented `get_live_telemetry()` endpoint
- [ ] Registered all routes in HTTP server
- [ ] Tested all endpoints manually with `curl`
- [ ] Verified JSON responses format
- [ ] Tested error cases (not found, lock failed, invalid state)
- [ ] `cargo build` passes
- [ ] `cargo clippy` passes

---

## Summary

Phase 3 provides a complete REST API for the frontend:
- ✅ Robot registration and lifecycle management
- ✅ Match control (start/stop/pause/resume)
- ✅ E-stop capabilities
- ✅ Real-time monitoring of robot status
- ✅ Consistent JSON responses with proper error handling

After Phase 3, the frontend can fully control and monitor the FMS system.

