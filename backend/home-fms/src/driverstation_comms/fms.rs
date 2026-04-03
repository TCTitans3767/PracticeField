use crate::driverstation_comms::driverstation_connection::{DSControl, DriverstationConnection};

#[derive(Default)]
pub struct FMS {
    pub driverstations: Vec<DriverstationConnection>,
    pub allowed_driverstations: Vec<u16>,
}

impl FMS {
    pub fn add_ds(&mut self, team_number: u16) {
        if self.allowed_driverstations.len() == 0 {
            if !self
                .driverstations
                .iter()
                .any(|driverstation| driverstation.team_number == team_number)
            {
                println!("added ds for team {team_number}");
                self.driverstations
                    .push(DriverstationConnection::new(team_number));
            }
        } else {
            if self.allowed_driverstations.contains(&team_number)
                && !self
                    .driverstations
                    .iter()
                    .any(|driverstation| driverstation.team_number == team_number)
            {
                self.driverstations
                    .push(DriverstationConnection::new(team_number));
            }
        }
    }

    pub fn remove_ds(&mut self, team_number: u16) {
        if let Some(driverstation_position) = self
            .driverstations
            .iter_mut()
            .position(|driverstation| driverstation.team_number == team_number)
        {
            self.driverstations.remove(driverstation_position);
        }
    }

    pub fn add_to_match(&mut self, team_number: u16, station: u8) {
        self.add_ds(team_number);
        self.allowed_driverstations.push(team_number);
        if let Some(ds) = self.get_ds(team_number) {
            ds.alliance_station = station;
            ds.ds_control = DSControl::FMSFull;
        }
    }

    pub fn get_ds(&mut self, team_number: u16) -> Option<&mut DriverstationConnection> {
        self.driverstations.iter_mut().find(|ds| {ds.team_number == team_number})
    }

    // pub fn delete_match

    pub fn enable_team() -> anyhow::Result<()> {
        Ok(())
    }
}
