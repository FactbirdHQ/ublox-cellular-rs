//! Responses for System features Commands
use super::types::*;
use atat::atat_derive::AtatResp;

/// 19.8 Power saving control (Power SaVing) +UPSV
#[derive(AtatResp)]
pub struct PowerSavingControl {
    #[at_arg(position = 0)]
    pub mode: PowerSavingMode,
    #[at_arg(position = 1)]
    pub timeout: Option<Seconds>,
}
