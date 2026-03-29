use crate::driverstation_comms::udp::DSUDPConnection;

pub struct DriverstationConnection {
    team_number: u16,
    driverstation_udp: DSUDPConnection,
    ds_control: DSControl
}

pub enum DSControl {
    Uncontrolled,
    FMSFull,
    FMSPartial,
}
