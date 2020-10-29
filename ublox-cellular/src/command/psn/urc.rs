//! Unsolicited responses for Packet Switched Data Services Commands
use super::types::*;
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

/// 18.27 GPRS network registration status +CGREG
#[derive(Clone, AtatResp)]
pub struct GPRSNetworkRegistration {
    #[at_arg(position = 1)]
    pub stat: GPRSNetworkRegistrationStat,
    // #[at_arg(position = 2)]
    // pub lac: Option<String<consts::U32>>,
    // #[at_arg(position = 3)]
    // pub ci: Option<String<consts::U32>>,
    // #[at_arg(position = 4)]
    // pub act_status: Option<u8>,
}

/// 18.28 Extended network registration status +UREG
#[derive(Clone, AtatResp)]
pub struct ExtendedPSNetworkRegistration {
    #[at_arg(position = 1)]
    pub state: ExtendedPSNetworkRegistrationState,
}

/// 18.36 EPS network registration status +CEREG
#[derive(Clone, AtatResp)]
pub struct EPSNetworkRegistration {
    #[at_arg(position = 1)]
    pub stat: EPSNetworkRegistrationStat,
    // #[at_arg(position = 2)]
    // pub lac: Option<String<consts::U32>>,
    // #[at_arg(position = 3)]
    // pub ci: Option<String<consts::U32>>,
    // #[at_arg(position = 4)]
    // pub act_status: Option<u8>,
}
