use std::sync::OnceLock;

use crate::driverstation_comms::driverstation_connection::DriverstationConnection;

pub struct FMS {
    pub driverstations: Vec<DriverstationConnection>,
}

impl FMS {
    pub fn get() -> &'static Self {
        static INSTANCE: OnceLock<FMS> = OnceLock::new();
        INSTANCE.get_or_init(|| FMS {
            driverstations: Vec::new(),
        })
    }

    pub fn add_ds(&mut self, team_number: u16) {
        self.driverstations.push(DriverstationConnection::new(team_number));
    }

    pub fn enable_team() -> anyhow::Result<()>{



        Ok(())
    }
}
