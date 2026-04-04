//! FMS State and Command Queue
//!
//! This module combines FMS state management with a command queue pattern.
//! All state changes are processed sequentially on a dedicated task,
//! eliminating the need to pass Arc<Mutex<FMS>> throughout the codebase.
//!
//! ## Architecture
//! - `FMS` struct holds all state (driverstations, allowed teams)
//! - `FMSCommandQueue` sends commands to a single processor task
//! - Access globally via `get_fms_queue()` after initialization
//!
//! ## Usage
//! ```ignore
//! // Initialize once at startup
//! init_fms_queue();
//!
//! // Access from anywhere
//! let queue = get_fms_queue();
//! queue.add_driver_station(1234).await;
//! ```

use std::sync::OnceLock;

use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use super::driverstation_connection::{DSControl, DriverstationConnection};
use super::udp::ControlMode;

// ============================================================================
// FMS State
// ============================================================================

/// Central FMS state holding all driverstation connections and match configuration
#[derive(Default)]
pub struct FMS {
    pub driverstations: Vec<DriverstationConnection>,
    pub allowed_driverstations: Vec<u16>,
}

impl FMS {
    /// Add a new driverstation connection (prevents duplicates)
    pub fn add_ds(&mut self, team_number: u16) {
        if self.allowed_driverstations.is_empty() {
            // Open mode: allow any team
            if !self
                .driverstations
                .iter()
                .any(|ds| ds.team_number == team_number)
            {
                info!("Added driverstation for team {team_number}");
                self.driverstations
                    .push(DriverstationConnection::new(team_number));
            }
        } else {
            // Restricted mode: only allow teams in match
            if self.allowed_driverstations.contains(&team_number)
                && !self
                    .driverstations
                    .iter()
                    .any(|ds| ds.team_number == team_number)
            {
                self.driverstations
                    .push(DriverstationConnection::new(team_number));
            }
        }
    }

    /// Remove a driverstation connection
    pub fn remove_ds(&mut self, team_number: u16) {
        if let Some(pos) = self
            .driverstations
            .iter()
            .position(|ds| ds.team_number == team_number)
        {
            self.driverstations.remove(pos);
        }
    }

    /// Add a team to the match with alliance station assignment
    pub fn add_to_match(&mut self, team_number: u16, station: u8) {
        self.allowed_driverstations.push(team_number);
        self.add_ds(team_number);
        if let Some(ds) = self.get_ds(team_number) {
            ds.alliance_station = station;
            ds.ds_control = DSControl::FMSFull;
        }
    }

    /// Get mutable reference to a driverstation
    pub fn get_ds(&mut self, team_number: u16) -> Option<&mut DriverstationConnection> {
        self.driverstations
            .iter_mut()
            .find(|ds| ds.team_number == team_number)
    }

    /// Get immutable reference to a driverstation
    pub fn get_ds_immutable(&self, team_number: u16) -> Option<&DriverstationConnection> {
        self.driverstations
            .iter()
            .find(|ds| ds.team_number == team_number)
    }

    /// Enable a team's robot
    pub fn enable_robot(&mut self, team_number: u16) {
        if let Some(ds) = self.get_ds(team_number) {
            ds.control_mode = [ds.control_mode[0].clone(), ControlMode::Enabled];
        }
    }

    /// Disable a team's robot
    pub fn disable_robot(&mut self, team_number: u16) {
        if let Some(ds) = self.get_ds(team_number) {
            ds.control_mode = [ds.control_mode[0].clone(), ControlMode::Disabled];
        }
    }

    /// Set robot control mode
    pub fn set_control_mode(&mut self, team_number: u16, mode: ControlModeCommand) {
        if let Some(ds) = self.get_ds(team_number) {
            ds.control_mode = match mode {
                ControlModeCommand::TeleopEnabled => [ControlMode::Teleop, ControlMode::Enabled],
                ControlModeCommand::TeleopDisabled => {
                    [ControlMode::Teleop, ControlMode::Disabled]
                }
                ControlModeCommand::AutonomousEnabled => {
                    [ControlMode::Autonomous, ControlMode::Enabled]
                }
                ControlModeCommand::AutonomousDisabled => {
                    [ControlMode::Autonomous, ControlMode::Disabled]
                }
                ControlModeCommand::TestEnabled => [ControlMode::Test, ControlMode::Enabled],
                ControlModeCommand::TestDisabled => [ControlMode::Test, ControlMode::Disabled],
                ControlModeCommand::EStop => [ControlMode::EStop, ControlMode::Disabled],
                ControlModeCommand::AStop => [ControlMode::AStop, ControlMode::Disabled],
            };
        }
    }

    /// Clear match configuration (keep connections, clear allowed list)
    pub fn clear_match(&mut self) {
        self.allowed_driverstations.clear();
        for ds in &mut self.driverstations {
            ds.ds_control = DSControl::FMSPartial;
        }
    }

    /// Reset entire FMS state
    pub fn reset(&mut self) {
        *self = FMS::default();
    }
}

// ============================================================================
// Commands
// ============================================================================

/// Control mode commands for robot operation
#[derive(Debug, Clone, Copy)]
pub enum ControlModeCommand {
    TeleopEnabled,
    TeleopDisabled,
    AutonomousEnabled,
    AutonomousDisabled,
    TestEnabled,
    TestDisabled,
    EStop,
    AStop,
}

/// Commands that can be sent to the FMS state processor
#[derive(Debug)]
pub enum FMSCommand {
    /// Add a new driverstation connection
    AddDriverStation { team_number: u16 },
    /// Remove a driverstation connection
    RemoveDriverStation { team_number: u16 },
    /// Add a team to the match with alliance station assignment
    AddToMatch {
        team_number: u16,
        station: u8,
    },
    /// Enable a specific team's robot
    EnableRobot { team_number: u16 },
    /// Disable a specific team's robot
    DisableRobot { team_number: u16 },
    /// Set robot control mode (Teleop/Auto/Test)
    SetControlMode {
        team_number: u16,
        mode: ControlModeCommand,
    },
    /// Query current FMS state snapshot
    GetState {
        response: oneshot::Sender<FMSStateSnapshot>,
    },
    /// Query specific driverstation info (returns cloned DriverstationConnection)
    GetDriverStation {
        team_number: u16,
        response: oneshot::Sender<Option<DriverstationConnection>>,
    },
    /// Get list of all connected teams
    GetAllTeams {
        response: oneshot::Sender<Vec<u16>>,
    },
    /// Clear match configuration
    ClearMatch,
    /// Reset entire FMS state
    Reset,
}

/// Snapshot of FMS state for queries
#[derive(Debug, Clone)]
pub struct FMSStateSnapshot {
    pub connected_teams: Vec<u16>,
    pub allowed_teams: Vec<u16>,
    pub total_driverstations: usize,
}

// ============================================================================
// Command Queue
// ============================================================================

/// FMS Command Queue - send commands to the FMS state processor
pub struct FMSCommandQueue {
    tx: mpsc::Sender<FMSCommand>,
}

impl FMSCommandQueue {
    /// Create a new command queue and spawn the processor task
    fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(100);

        // Spawn high-priority state processor task
        tokio::spawn(async move {
            let mut fms = FMS::default();

            info!("FMS command processor started");

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    FMSCommand::AddDriverStation { team_number } => {
                        debug!("FMS: Adding driverstation for team {}", team_number);
                        fms.add_ds(team_number);
                    }

                    FMSCommand::RemoveDriverStation { team_number } => {
                        debug!("FMS: Removing driverstation for team {}", team_number);
                        fms.remove_ds(team_number);
                    }

                    FMSCommand::AddToMatch {
                        team_number,
                        station,
                    } => {
                        debug!(
                            "FMS: Adding team {} to match at station {}",
                            team_number, station
                        );
                        fms.add_to_match(team_number, station);
                    }

                    FMSCommand::EnableRobot { team_number } => {
                        debug!("FMS: Enabling robot for team {}", team_number);
                        fms.enable_robot(team_number);
                    }

                    FMSCommand::DisableRobot { team_number } => {
                        debug!("FMS: Disabling robot for team {}", team_number);
                        fms.disable_robot(team_number);
                    }

                    FMSCommand::SetControlMode {
                        team_number,
                        mode,
                    } => {
                        debug!("FMS: Setting control mode {:?} for team {}", mode, team_number);
                        fms.set_control_mode(team_number, mode);
                    }

                    FMSCommand::GetState { response } => {
                        let snapshot = FMSStateSnapshot {
                            connected_teams: fms
                                .driverstations
                                .iter()
                                .map(|ds| ds.team_number)
                                .collect(),
                            allowed_teams: fms.allowed_driverstations.clone(),
                            total_driverstations: fms.driverstations.len(),
                        };
                        let _ = response.send(snapshot);
                    }

                    FMSCommand::GetDriverStation {
                        team_number,
                        response,
                    } => {
                        let info = fms.get_ds_immutable(team_number).cloned();
                        let _ = response.send(info);
                    }

                    FMSCommand::GetAllTeams { response } => {
                        let teams: Vec<u16> =
                            fms.driverstations.iter().map(|ds| ds.team_number).collect();
                        let _ = response.send(teams);
                    }

                    FMSCommand::ClearMatch => {
                        debug!("FMS: Clearing match configuration");
                        fms.clear_match();
                    }

                    FMSCommand::Reset => {
                        debug!("FMS: Resetting all state");
                        fms.reset();
                    }
                }
            }

            error!("FMS command processor stopped");
        });

        Self { tx }
    }

    /// Send a command to the FMS state processor
    pub async fn send(&self, command: FMSCommand) {
        if let Err(e) = self.tx.send(command).await {
            error!("Failed to send FMS command: {}", e);
        }
    }

    // ========================================================================
    // Convenience Methods
    // ========================================================================

    /// Add a driverstation connection
    pub async fn add_driver_station(&self, team_number: u16) {
        self.send(FMSCommand::AddDriverStation { team_number })
            .await;
    }

    /// Remove a driverstation connection
    pub async fn remove_driver_station(&self, team_number: u16) {
        self.send(FMSCommand::RemoveDriverStation { team_number })
            .await;
    }

    /// Add a team to the match with alliance station
    pub async fn add_to_match(&self, team_number: u16, station: u8) {
        self.send(FMSCommand::AddToMatch {
            team_number,
            station,
        })
        .await;
    }

    /// Enable a team's robot
    pub async fn enable_robot(&self, team_number: u16) {
        self.send(FMSCommand::EnableRobot { team_number }).await;
    }

    /// Disable a team's robot
    pub async fn disable_robot(&self, team_number: u16) {
        self.send(FMSCommand::DisableRobot { team_number }).await;
    }

    /// Set robot control mode
    pub async fn set_control_mode(&self, team_number: u16, mode: ControlModeCommand) {
        self.send(FMSCommand::SetControlMode { team_number, mode })
            .await;
    }

    /// Get current FMS state snapshot
    pub async fn get_state(&self) -> FMSStateSnapshot {
        let (tx, rx) = oneshot::channel();
        self.send(FMSCommand::GetState { response: tx }).await;
        rx.await.expect("FMS state processor dropped")
    }

    /// Get info for a specific driverstation
    pub async fn get_driver_station(
        &self,
        team_number: u16,
    ) -> Option<DriverstationConnection> {
        let (tx, rx) = oneshot::channel();
        self.send(FMSCommand::GetDriverStation {
            team_number,
            response: tx,
        })
        .await;
        rx.await.expect("FMS state processor dropped")
    }

    /// Get list of all connected teams
    pub async fn get_all_teams(&self) -> Vec<u16> {
        let (tx, rx) = oneshot::channel();
        self.send(FMSCommand::GetAllTeams { response: tx }).await;
        rx.await.expect("FMS state processor dropped")
    }

    /// Clear match configuration
    pub async fn clear_match(&self) {
        self.send(FMSCommand::ClearMatch).await;
    }

    /// Reset all FMS state
    pub async fn reset(&self) {
        self.send(FMSCommand::Reset).await;
    }
}

// ============================================================================
// Global Static Access
// ============================================================================

static FMS_QUEUE: OnceLock<FMSCommandQueue> = OnceLock::new();

/// Initialize the FMS command queue (call once at startup)
pub fn init_fms_queue() -> &'static FMSCommandQueue {
    FMS_QUEUE.get_or_init(FMSCommandQueue::new)
}

/// Get the global FMS command queue (panics if not initialized)
pub fn get_fms_queue() -> &'static FMSCommandQueue {
    FMS_QUEUE
        .get()
        .expect("FMS queue not initialized. Call init_fms_queue() first.")
}

/// Try to get the global FMS command queue (returns None if not initialized)
pub fn try_get_fms_queue() -> Option<&'static FMSCommandQueue> {
    FMS_QUEUE.get()
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_fms_queue() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;

        let state = queue.get_state().await;
        assert!(state.connected_teams.contains(&1234));
        assert_eq!(state.total_driverstations, 1);
    }

    #[tokio::test]
    async fn test_add_driver_station() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(254).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = queue.get_state().await;
        assert!(state.connected_teams.contains(&254));
        assert_eq!(state.total_driverstations, 1);
    }

    #[tokio::test]
    async fn test_add_multiple_driver_stations() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        queue.add_driver_station(254).await;
        queue.add_driver_station(1).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = queue.get_state().await;
        assert_eq!(state.total_driverstations, 3);
        assert!(state.connected_teams.contains(&1234));
        assert!(state.connected_teams.contains(&254));
        assert!(state.connected_teams.contains(&1));
    }

    #[tokio::test]
    async fn test_remove_driver_station() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        queue.add_driver_station(254).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.remove_driver_station(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = queue.get_state().await;
        assert_eq!(state.total_driverstations, 1);
        assert!(!state.connected_teams.contains(&1234));
        assert!(state.connected_teams.contains(&254));
    }

    #[tokio::test]
    async fn test_add_to_match() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_to_match(1234, 3).await; // Blue 1
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = queue.get_state().await;
        assert!(state.connected_teams.contains(&1234));
        assert!(state.allowed_teams.contains(&1234));

        let ds_info = queue.get_driver_station(1234).await;
        assert!(ds_info.is_some());
        assert_eq!(ds_info.unwrap().alliance_station, 3);
    }

    #[tokio::test]
    async fn test_enable_disable_robot() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.enable_robot(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds_info = queue.get_driver_station(1234).await.unwrap();
        assert!(matches!(ds_info.control_mode[1], ControlMode::Enabled));

        queue.disable_robot(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds_info = queue.get_driver_station(1234).await.unwrap();
        assert!(matches!(ds_info.control_mode[1], ControlMode::Disabled));
    }

    #[tokio::test]
    async fn test_set_control_mode_teleop() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue
            .set_control_mode(1234, ControlModeCommand::TeleopEnabled)
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds_info = queue.get_driver_station(1234).await.unwrap();
        assert!(matches!(ds_info.control_mode[0], ControlMode::Teleop));
        assert!(matches!(ds_info.control_mode[1], ControlMode::Enabled));
    }

    #[tokio::test]
    async fn test_set_control_mode_autonomous() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue
            .set_control_mode(1234, ControlModeCommand::AutonomousEnabled)
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds_info = queue.get_driver_station(1234).await.unwrap();
        assert!(matches!(ds_info.control_mode[0], ControlMode::Autonomous));
        assert!(matches!(ds_info.control_mode[1], ControlMode::Enabled));
    }

    #[tokio::test]
    async fn test_set_control_mode_estop() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue
            .set_control_mode(1234, ControlModeCommand::EStop)
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds_info = queue.get_driver_station(1234).await.unwrap();
        assert!(matches!(ds_info.control_mode[0], ControlMode::EStop));
    }

    #[tokio::test]
    async fn test_clear_match() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_to_match(1234, 3).await;
        queue.add_to_match(254, 0).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state_before = queue.get_state().await;
        assert_eq!(state_before.allowed_teams.len(), 2);

        queue.clear_match().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state_after = queue.get_state().await;
        assert_eq!(state_after.allowed_teams.len(), 0);
        assert_eq!(state_after.total_driverstations, 2);
    }

    #[tokio::test]
    async fn test_reset() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_to_match(1234, 3).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state_before = queue.get_state().await;
        assert_eq!(state_before.total_driverstations, 1);

        queue.reset().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state_after = queue.get_state().await;
        assert_eq!(state_after.total_driverstations, 0);
        assert_eq!(state_after.allowed_teams.len(), 0);
    }

    #[tokio::test]
    async fn test_get_all_teams() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        queue.add_driver_station(254).await;
        queue.add_driver_station(1).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let teams = queue.get_all_teams().await;
        assert_eq!(teams.len(), 3);
        assert!(teams.contains(&1234));
        assert!(teams.contains(&254));
        assert!(teams.contains(&1));
    }

    #[tokio::test]
    async fn test_get_nonexistent_driver_station() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds_info = queue.get_driver_station(9999).await;
        assert!(ds_info.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_driver_station_prevention() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_driver_station(1234).await;
        queue.add_driver_station(1234).await;
        queue.add_driver_station(1234).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let state = queue.get_state().await;
        assert_eq!(state.total_driverstations, 1);
    }

    #[tokio::test]
    async fn test_get_driver_station_returns_clone() {
        let queue = FMSCommandQueue::new();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        queue.add_to_match(1234, 3).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let ds = queue.get_driver_station(1234).await.unwrap();
        assert_eq!(ds.team_number, 1234);
        assert_eq!(ds.alliance_station, 3);
        assert!(matches!(ds.ds_control, DSControl::FMSFull));
    }
}
