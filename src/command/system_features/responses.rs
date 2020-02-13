//! 4 Responses for General Commands
use super::types::*;
use atat::atat_derive::ATATResp;
use atat::ATATResp;

/// 19.8 Power saving control (Power SaVing) +UPSV
#[derive(ATATResp)]
pub struct PowerSavingControl {
    #[at_arg(position = 0)]
    mode: PowerSavingMode,
    #[at_arg(position = 1)]
    timeout: Option<Seconds>,
}
