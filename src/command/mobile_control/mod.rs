//! 4 General Commands
pub mod responses;
pub mod types;
use at::{Error, ATATCmd};
use heapless::{consts, String};
use responses::*;
use types::*;
use at::atat_derive::ATATCmd;
use serde::Serialize;
use super::NoResponse;



/// 5.3 Set module functionality +CFUN
/// Selects the level of functionality <fun> in the MT.
// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CFUN", NoResponse, timeout_ms = 180000)]
pub struct SetModuleFunctionality {
    #[atat_(position = 0)]
    fun: Functionality,
    #[atat_(position = 1)]
    rst: Option<ResetMode>,
}

// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CFUN?", ModuleFunctionality, timeout_ms = 180000)]
pub struct GetModuleFunctionality;

/// 5.19 Report mobile termination error +CMEE
/// Configures the formatting of the result code +CME ERROR: <err> as an indication of an error relating to the
/// functionality of the MT. When enabled, MT related errors cause +CME ERROR: <err> final result code instead
/// of the regular ERROR final result code. The error result code is returned normally when an error is related to
/// syntax, invalid parameters or MT functionality
// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CMEE", NoResponse)]
pub struct SetReportMobileTerminationError {
    #[atat_(position = 0)]
    n: TerminationErrorMode,
}

// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CMEE?", ReportMobileTerminationError)]
pub struct GetReportMobileTerminationError;

