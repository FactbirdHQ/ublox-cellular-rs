//! Unsolicited responses for Packet Switched Data Services Commands
use super::types::{
    EPSNetworkRegistrationStat, ExtendedPSNetworkRegistrationState, GPRSNetworkRegistrationStat,
};
use crate::{command::network_service::types::RatAct, network::ProfileId};
use atat::atat_derive::AtatResp;
use embedded_nal::IpAddr;
use heapless::String;

/// +UUPSDA
#[derive(Debug, Clone, AtatResp)]
pub struct DataConnectionActivated {
    #[at_arg(position = 0)]
    pub result: u8,
    #[at_arg(position = 1, len = 39)]
    pub ip_addr: Option<IpAddr>,
}

/// +UUPSDD
#[derive(Debug, Clone, AtatResp)]
pub struct DataConnectionDeactivated {
    #[at_arg(position = 0)]
    pub profile_id: ProfileId,
}

/// 18.27 GPRS network registration status +CGREG
#[derive(Debug, Clone, AtatResp)]
pub struct GPRSNetworkRegistration {
    #[at_arg(position = 1)]
    pub stat: GPRSNetworkRegistrationStat,
    #[at_arg(position = 2)]
    pub lac: Option<String<4>>,
    #[at_arg(position = 3)]
    pub ci: Option<String<8>>,
    #[at_arg(position = 4)]
    pub act: Option<RatAct>,
    #[at_arg(position = 5)]
    pub rac: Option<String<2>>,
}

/// 18.28 Extended network registration status +UREG
#[derive(Debug, Clone, AtatResp)]
pub struct ExtendedPSNetworkRegistration {
    #[at_arg(position = 1)]
    pub state: ExtendedPSNetworkRegistrationState,
}

/// 18.36 EPS network registration status +CEREG
#[derive(Debug, Clone, AtatResp)]
pub struct EPSNetworkRegistration {
    #[at_arg(position = 1)]
    pub stat: EPSNetworkRegistrationStat,
    #[at_arg(position = 2)]
    pub tac: Option<String<4>>,
    #[at_arg(position = 3)]
    pub ci: Option<String<8>>,
    #[at_arg(position = 4)]
    pub act: Option<RatAct>,
    #[at_arg(position = 5)]
    pub cause_type: Option<u8>,
    #[at_arg(position = 6)]
    pub reject_cause: Option<u8>,
}
