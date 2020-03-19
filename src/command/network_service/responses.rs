//! Responses for Network service Commands
use super::types::*;
use atat::atat_derive::AtatResp;

/// 7.14 Network registration status +CREG
#[derive(Clone, AtatResp)]
pub struct NetworkRegistrationStatus {
    #[at_arg(position = 0)]
    pub n: NetworkRegistrationUrc,
    #[at_arg(position = 1)]
    pub stat: NetworkRegistrationStat,
    // #[at_arg(position = 2)]
    // pub lac: Option<String<consts::U32>>,
    // #[at_arg(position = 3)]
    // pub ci: Option<String<consts::U32>>,
    // #[at_arg(position = 4)]
    // pub act_status: Option<u8>,
}
