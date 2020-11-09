//! Responses for Network service Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 7.5 Operator selection +COPS
#[derive(Clone, AtatResp)]
pub struct OperatorSelection {
    #[at_arg(position = 0)]
    pub mode: OperatorSelectionMode,
    #[at_arg(position = 1)]
    pub format: Option<u8>,
    #[at_arg(position = 2)]
    pub oper: Option<String<consts::U24>>,
    #[at_arg(position = 3)]
    pub act: Option<u8>,
}

/// 7.8 Radio Access Technology (RAT) selection +URAT
#[derive(Clone, AtatResp)]
pub struct RadioAccessTechnology {
    #[at_arg(position = 0)]
    pub act: RadioAccessTechnologySelected,
}

/// 7.14 Network registration status +CREG
#[derive(Clone, AtatResp)]
pub struct NetworkRegistrationStatus {
    #[at_arg(position = 0)]
    pub n: NetworkRegistrationUrcConfig,
    #[at_arg(position = 1)]
    pub stat: NetworkRegistrationStat,
    // #[at_arg(position = 2)]
    // pub lac: Option<String<consts::U32>>,
    // #[at_arg(position = 3)]
    // pub ci: Option<String<consts::U32>>,
    // #[at_arg(position = 4)]
    // pub act_status: Option<u8>,
}
