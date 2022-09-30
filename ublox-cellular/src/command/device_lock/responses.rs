//! Responses for Device lock Commands
use super::types::*;
use atat::atat_derive::AtatResp;

/// 9.1 Enter PIN +CPIN
#[derive(Clone, Debug, PartialEq, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PinStatus {
    #[at_arg(position = 0)]
    pub code: PinStatusCode,
}
