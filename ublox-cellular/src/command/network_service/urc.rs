//! Unsolicited responses for Network service Commands
use super::NetworkRegistrationStat;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 7.14 Network registration status +CREG
#[derive(Debug, Clone, AtatResp)]
pub struct NetworkRegistration {
    #[at_arg(position = 1)]
    pub stat: NetworkRegistrationStat,
    #[at_arg(position = 2)]
    pub lac: Option<String<consts::U4>>,
    #[at_arg(position = 3)]
    pub ci: Option<String<consts::U8>>,
    #[at_arg(position = 4)]
    pub act_status: Option<u8>,
}
