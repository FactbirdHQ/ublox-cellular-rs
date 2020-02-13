//! 4 General Commands
pub mod responses;
pub mod types;
use atat::{Error, atat_derive::ATATCmd, ATATCmd};
use heapless::{consts, String, Vec};
use responses::*;
use types::*;

use super::NoResponse;


/// 5.3 Set module functionality +CFUN
/// Selects the level of functionality <fun> in the MT.
#[derive(Clone, ATATCmd)]
#[at_cmd("+CFUN", NoResponse, timeout_ms = 180000)]
pub struct SetModuleFunctionality {
    #[at_arg(position = 0)]
    pub fun: Functionality,
    #[at_arg(position = 1)]
    pub rst: Option<ResetMode>,
}

#[derive(Clone, ATATCmd)]
#[at_cmd("+CFUN?", ModuleFunctionality, timeout_ms = 180000)]
pub struct GetModuleFunctionality;

/// 5.19 Report mobile termination error +CMEE
/// Configures the formatting of the result code +CME ERROR: <err> as an indication of an error relating to the
/// functionality of the MT. When enabled, MT related errors cause +CME ERROR: <err> final result code instead
/// of the regular ERROR final result code. The error result code is returned normally when an error is related to
/// syntax, invalid parameters or MT functionality
#[derive(Clone, ATATCmd)]
#[at_cmd("+CMEE", NoResponse)]
pub struct SetReportMobileTerminationError {
    #[at_arg(position = 0)]
    pub n: TerminationErrorMode,
}

#[derive(Clone, ATATCmd)]
#[at_cmd("+CMEE?", ReportMobileTerminationError)]
pub struct GetReportMobileTerminationError;
