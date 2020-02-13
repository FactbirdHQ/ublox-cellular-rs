//! 4 Responses for General Commands
use super::types::*;
use atat::atat_derive::ATATResp;
use atat::ATATResp;
use heapless::{consts, Vec};

/// 5.3 Set module functionality +CFUN
/// Selects the level of functionality <fun> in the MT.
#[derive(Clone, ATATResp)]
pub struct ModuleFunctionality {
    #[at_arg(position = 0)]
    pub power_mode: PowerMode,
    #[at_arg(position = 1)]
    pub stk_mode: STKMode,
}

/// 5.19 Report mobile termination error +CMEE
/// Configures the formatting of the result code +CME ERROR: <err> as an indication of an error relating to the
/// functionality of the MT. When enabled, MT related errors cause +CME ERROR: <err> final result code instead
/// of the regular ERROR final result code. The error result code is returned normally when an error is related to
/// syntax, invalid parameters or MT functionality
#[derive(Clone, ATATResp)]
pub struct ReportMobileTerminationError {
    #[at_arg(position = 0)]
    pub status: ReportMobileTerminationErrorStatus,
}
