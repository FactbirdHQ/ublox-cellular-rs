//! 4 Responses for General Commands
use heapless::{consts, String};
use serde::Deserialize;



/// 5.3 Set module functionality +CFUN
/// Selects the level of functionality <fun> in the MT.
/// // #[derive(Deserialize)]
pub struct ModuleFunctionality{
    // #[atat_(position = 0)]
    pub powerMode : PowerMode,
    // #[atat_(position = 1)]
    pub stkMode : STKMode,
}



/// 5.19 Report mobile termination error +CMEE
/// Configures the formatting of the result code +CME ERROR: <err> as an indication of an error relating to the
/// functionality of the MT. When enabled, MT related errors cause +CME ERROR: <err> final result code instead
/// of the regular ERROR final result code. The error result code is returned normally when an error is related to
/// syntax, invalid parameters or MT functionality
// #[derive(Deserialize)]
pub struct ReportMobileTerminationError{
    // #[atat_(position = 0)]
    pub status : ReportMobileTerminationErrorStatus
}