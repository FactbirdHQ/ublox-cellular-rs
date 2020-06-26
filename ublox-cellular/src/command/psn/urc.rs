//! Unsolicited responses for Packet Switched Data Services Commands
use atat::atat_derive::AtatResp;

#[derive(Clone, AtatResp)]
pub struct DataConnectionActivated {
    #[at_arg(position = 0)]
    pub result: u8,
}

#[derive(Clone, AtatResp)]
pub struct DataConnectionDeactivated {
    #[at_arg(position = 0)]
    pub profile_id: u8,
}
