# Phase 5: Field Radio Integration (Optional Future Enhancement)

**Objective:** Configure field radio to enable/disable team networks in sync with FMS match state.

**Duration:** ~1.5 hours (if radio hardware available)  
**Status:** Depends on Phase 3 completion  
**Depends On:** P3.1-P3.6 (API framework)  
**Prerequisites:** Field radio hardware with network configuration capability

---

## Overview

Phase 5 is optional and depends on having actual field radio hardware that supports remote network configuration. This phase integrates the FMS with the field radio so that:
- When a match starts, the radio enables the team networks for enabled robots
- When a match stops, the radio disables the team networks
- The FMS tracks radio status and alerts on misconfiguration

If you don't have field radio hardware, this phase can be skipped or postponed indefinitely.

---

## P5.1: Field Radio Configuration Protocol

### Understanding Field Radio Configuration

FIRST field radios (such as the Cisco or newer models) support:
- **mDNS discovery** - Finding the radio on the network
- **HTTP REST API** - Configuring team networks
- **SSH access** - Advanced configuration
- **Network configuration** - Enable/disable specific team SSIDs

### Solution

**Step 1: Create Radio Configuration Module**

Create `src/radio.rs`:

```rust
use reqwest::{Client, StatusCode};
use tracing::{info, warn, error, debug};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Configuration for field radio connection
#[derive(Debug, Clone)]
pub struct RadioConfig {
    /// IP address or hostname of field radio
    pub host: String,
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
    /// HTTP timeout in seconds
    pub timeout_secs: u64,
    /// Retry count for failed requests
    pub max_retries: u32,
}

/// Team network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamNetwork {
    pub team_number: u16,
    pub ssid: String,
    pub enabled: bool,
}

/// Radio status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioStatus {
    pub connected: bool,
    pub firmware_version: Option<String>,
    pub configured_teams: Vec<u16>,
    pub last_sync_ms: Option<i64>,
}

/// Radio client for communicating with field radio
pub struct RadioClient {
    config: RadioConfig,
    client: Client,
}

impl RadioClient {
    pub fn new(config: RadioConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client for radio")?;

        info!(host = &config.host, "Radio client initialized");

        Ok(Self { config, client })
    }

    /// Check if radio is accessible
    pub async fn health_check(&self) -> Result<bool> {
        for attempt in 0..self.config.max_retries {
            match self.client
                .get(&format!("http://{}/api/status", self.config.host))
                .send()
                .await
            {
                Ok(resp) => {
                    debug!("Radio health check successful");
                    return Ok(resp.status() == StatusCode::OK);
                }
                Err(e) => {
                    warn!(attempt, error = %e, "Radio health check failed");
                    if attempt < self.config.max_retries - 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        error!("Radio health check failed after {} attempts", self.config.max_retries);
        Ok(false)
    }

    /// Get current radio status
    pub async fn get_status(&self) -> Result<RadioStatus> {
        let resp = self.client
            .get(&format!("http://{}/api/status", self.config.host))
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .context("Failed to get radio status")?;

        let status: serde_json::Value = resp
            .json()
            .await
            .context("Failed to parse radio status response")?;

        // Parse based on actual radio API response format
        let configured_teams = status
            .get("configured_teams")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u16))
                    .collect()
            })
            .unwrap_or_default();

        Ok(RadioStatus {
            connected: true,
            firmware_version: status
                .get("firmware_version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            configured_teams,
            last_sync_ms: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
            ),
        })
    }

    /// Enable team networks on radio
    pub async fn enable_teams(&self, teams: &[u16]) -> Result<()> {
        let payload = serde_json::json!({
            "action": "enable_teams",
            "teams": teams
        });

        for attempt in 0..self.config.max_retries {
            match self.client
                .post(&format!("http://{}/api/configure", self.config.host))
                .basic_auth(&self.config.username, Some(&self.config.password))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status() == StatusCode::OK {
                        info!(team_count = teams.len(), "Teams enabled on radio");
                        return Ok(());
                    } else {
                        warn!(
                            status = %resp.status(),
                            attempt,
                            "Radio returned error while enabling teams"
                        );
                    }
                }
                Err(e) => {
                    warn!(error = %e, attempt, "Failed to enable teams on radio");
                }
            }

            if attempt < self.config.max_retries - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        anyhow::bail!("Failed to enable teams on radio after {} attempts", self.config.max_retries)
    }

    /// Disable all team networks on radio
    pub async fn disable_all_teams(&self) -> Result<()> {
        let payload = serde_json::json!({
            "action": "disable_all"
        });

        for attempt in 0..self.config.max_retries {
            match self.client
                .post(&format!("http://{}/api/configure", self.config.host))
                .basic_auth(&self.config.username, Some(&self.config.password))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status() == StatusCode::OK {
                        info!("All teams disabled on radio");
                        return Ok(());
                    }
                }
                Err(e) => {
                    warn!(error = %e, attempt, "Failed to disable teams on radio");
                }
            }

            if attempt < self.config.max_retries - 1 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        anyhow::bail!("Failed to disable teams on radio after {} attempts", self.config.max_retries)
    }
}
```

**Step 2: Add Radio Client to FMS**

Update `fms.rs`:

```rust
pub struct FMS {
    // ... existing fields
    pub radio_client: Option<Arc<RadioClient>>,  // ← Add this
    pub radio_last_sync: Option<std::time::Instant>,
}

impl FMS {
    pub fn sync_radio(&mut self, enabled_teams: Vec<u16>) -> anyhow::Result<()> {
        if let Some(radio) = &self.radio_client {
            tokio::spawn({
                let radio = radio.clone();
                async move {
                    if let Err(e) = radio.enable_teams(&enabled_teams).await {
                        error!(error = %e, "Failed to sync teams to radio");
                    } else {
                        info!(team_count = enabled_teams.len(), "Successfully synced teams to radio");
                    }
                }
            });

            self.radio_last_sync = Some(std::time::Instant::now());
        }

        Ok(())
    }
}
```

---

## P5.2: Radio State Sync Integration

### Solution

**Update match control endpoints to sync with radio:**

In `main.rs` interface module:

```rust
/// POST /api/match/start - Enhanced with radio sync
pub async fn start_match(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Starting match");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // ... existing enable logic ...

    let enabled_teams: Vec<u16> = fms_lock.driverstations
        .iter()
        .filter(|ds| ds.control_mode[1] == ControlMode::Enabled)
        .map(|ds| ds.team_number)
        .collect();

    // NEW: Sync enabled teams to radio
    if !enabled_teams.is_empty() {
        match fms_lock.sync_radio(enabled_teams.clone()) {
            Ok(_) => {
                info!(team_count = enabled_teams.len(), "Radio synced with enabled teams");
            }
            Err(e) => {
                warn!(error = %e, "Failed to sync radio - continuing with FMS");
                // Non-fatal error - FMS continues even if radio sync fails
            }
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "status": "match_started",
        "enabled_robots": enabled_teams,
        "count": enabled_teams.len()
    }))))
}

/// POST /api/match/stop - Enhanced with radio sync
pub async fn stop_match(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    info!("API: Stopping match");

    let mut fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    // ... existing disable logic ...

    // NEW: Disable all teams on radio
    if let Some(radio) = &fms_lock.radio_client {
        match radio.disable_all_teams().await {
            Ok(_) => {
                info!("Radio disabled all teams");
            }
            Err(e) => {
                warn!(error = %e, "Failed to disable teams on radio");
            }
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse::ok(json!({
        "status": "match_stopped",
        "all_robots_disabled": true
    }))))
}
```

**Add radio status monitoring endpoint:**

```rust
/// GET /api/radio/status - Get field radio status
pub async fn get_radio_status(
    fms: web::Data<Arc<Mutex<FMS>>>,
) -> Result<HttpResponse, ApiError> {
    let fms_lock = fms.lock()
        .map_err(|e| ApiError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("FMS lock failed: {}", e),
        })?;

    if let Some(radio) = &fms_lock.radio_client {
        match radio.get_status().await {
            Ok(status) => {
                Ok(HttpResponse::Ok().json(ApiResponse::ok(status)))
            }
            Err(e) => {
                Err(ApiError {
                    code: StatusCode::SERVICE_UNAVAILABLE,
                    message: format!("Radio communication failed: {}", e),
                })
            }
        }
    } else {
        Err(ApiError {
            code: StatusCode::NOT_FOUND,
            message: "Radio client not configured".to_string(),
        })
    }
}
```

---

## Configuration

In `main.rs`, optionally initialize radio client:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... initialization
    
    // Optionally configure field radio
    let radio_client = if let Ok(host) = std::env::var("FIELD_RADIO_HOST") {
        let radio_config = RadioConfig {
            host,
            username: std::env::var("FIELD_RADIO_USER")
                .unwrap_or_else(|_| "admin".to_string()),
            password: std::env::var("FIELD_RADIO_PASS")
                .unwrap_or_else(|_| "password".to_string()),
            timeout_secs: 5,
            max_retries: 3,
        };

        match RadioClient::new(radio_config) {
            Ok(client) => {
                info!("Field radio client configured");
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!(error = %e, "Failed to configure field radio - proceeding without radio support");
                None
            }
        }
    } else {
        debug!("FIELD_RADIO_HOST not set - radio integration disabled");
        None
    };

    // Add radio client to FMS
    fms.lock().unwrap().radio_client = radio_client;
}
```

**Environment variables for configuration:**

```bash
export FIELD_RADIO_HOST=10.0.100.5
export FIELD_RADIO_USER=admin
export FIELD_RADIO_PASS=secretpassword
export FIELD_RADIO_TIMEOUT_SECS=5
```

---

## Testing Radio Integration

```bash
# Check radio status
curl http://localhost:2000/api/radio/status

# Start match (syncs to radio)
curl -X POST http://localhost:2000/api/match/start

# Check radio configured teams
curl http://localhost:2000/api/radio/status

# Stop match (disables on radio)
curl -X POST http://localhost:2000/api/match/stop
```

---

## Phase 5 Completion Checklist

- [ ] Created `RadioClient` struct and module
- [ ] Implemented `health_check()` method
- [ ] Implemented `get_status()` method
- [ ] Implemented `enable_teams()` method
- [ ] Implemented `disable_all_teams()` method
- [ ] Added `radio_client` to `FMS`
- [ ] Updated `start_match()` to sync radio
- [ ] Updated `stop_match()` to disable radio
- [ ] Created `/api/radio/status` endpoint
- [ ] Registered routes in HTTP server
- [ ] Added environment variable configuration
- [ ] Tested with actual field radio (if available)
- [ ] Tested graceful degradation (FMS works without radio)
- [ ] Documented radio API requirements
- [ ] `cargo build` passes
- [ ] `cargo clippy` passes

---

## Notes

- **Optional Phase:** This phase is only necessary if you have field radio hardware
- **Graceful Degradation:** FMS continues to operate normally if radio is unavailable
- **Error Handling:** Radio sync failures are logged but don't prevent match operation
- **API Format:** The exact radio API endpoints depend on your specific field radio model

---

## Summary

Phase 5 provides optional field radio integration:
- ✅ Automatic team network configuration
- ✅ Sync between FMS and radio state
- ✅ Graceful degradation if radio is unavailable

Phase 5 is a nice-to-have enhancement that only applies if you have field radio hardware with network configuration capability.

