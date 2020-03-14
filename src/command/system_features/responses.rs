//! Responses for System features Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use atat::AtatResp;

/// 19.8 Power saving control (Power SaVing) +UPSV
#[derive(AtatResp)]
pub struct PowerSavingControl {
    #[at_arg(position = 0)]
    mode: PowerSavingMode,
    #[at_arg(position = 1)]
    timeout: Option<Seconds>,
}
