//! ### 5 - Mobile equipment control and status Commands
//!

pub mod responses;
pub mod types;
use atat::atat_derive::AtatCmd;
use responses::{DateTime, ExtendedErrorReport, IndicatorControl, ModuleFunctionality, ReportMobileTerminationError};
use types::{AutomaticTimezone, Functionality, ResetMode, TerminationErrorMode};

use super::NoResponse;

/// 5.2 Module switch off +CPWROFF
///
/// Switches off the MT. During shut-down current settings are saved in module's
/// non-volatile memory
///
/// **Notes:**
/// - Using this command can result in the following command line being ignored.
/// - See the corresponding System Integration Manual for the timing and the
///   electrical details of the module power-off sequence via the +CPWROFF
///   command.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CPWROFF", NoResponse, attempts = 1, timeout_ms = 40000)]
pub struct ModuleSwitchOff;

/// 5.3 Set module functionality +CFUN
///
/// Selects the level of functionality <fun> in the MT.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CFUN", NoResponse, attempts = 1, timeout_ms = 180000)]
pub struct SetModuleFunctionality {
    #[at_arg(position = 0)]
    pub fun: Functionality,
    #[at_arg(position = 1)]
    pub rst: Option<ResetMode>,
}

/// 5.3 Set module functionality +CFUN
///
/// Selects the level of functionality <fun> in the MT.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CFUN?", ModuleFunctionality, attempts = 1, timeout_ms = 8000)]
pub struct GetModuleFunctionality;

/// 5.4 Indicator control +CIND
///
/// Provides indication states related to network status, battery information
/// and so on. The set command does not allow setting the values for those
/// indications which are set according to module state (see <descr> parameter).
/// The list of indications for set and read commands follows the indexes
/// reported in the <descr> parameter, so that the first <ind> corresponds to
/// "battchg" and so on
#[derive(Clone, AtatCmd)]
#[at_cmd("+CIND?", IndicatorControl)]
pub struct GetIndicatorControl;

/// 5.7 Clock +CCLK
///
/// Sets the real-time clock of the MT
#[derive(Clone, AtatCmd)]
#[at_cmd("+CCLK", NoResponse)]
pub struct SetClock<'a> {
    #[at_arg(len = 20)]
    pub time: &'a str,
}

/// 5.7 Clock +CCLK
///
/// Reads the real-time clock of the MT
///
/// **Notes:**
/// - If the parameter value is out of range, then the "+CME ERROR: operation
///   not supported" or "+CME ERROR: 4" will be provided (depending on the +CMEE
///   AT command setting).
/// - "TZ": The Time Zone information is represented by two digits. The value is
///   updated during the registration procedure when the automatic time zone
///   update is enabled (using +CTZU command) and the network supports the time
///   zone information.
/// - The Time Zone information is expressed in steps of 15 minutes and it can
///   assume a value in the range that goes from -96 to +96.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CCLK?", DateTime)]
pub struct GetClock;

/// 5.15 Automatic time zone update +CTZU
///
/// Configures the automatic time zone update via NITZ.
///
/// **Notes:**
/// - The Time Zone information is provided after the network registration (if
///   the network supports the time zone information).
#[derive(Clone, AtatCmd)]
#[at_cmd("+CTZU", NoResponse)]
pub struct SetAutomaticTimezoneUpdate {
    #[at_arg(position = 0)]
    pub on_off: AutomaticTimezone,
}

/// 5.19 Report mobile termination error +CMEE
///
/// Configures the formatting of the result code +CME ERROR: <err> as an
/// indication of an error relating to the functionality of the MT. When
/// enabled, MT related errors cause +CME ERROR: <err> final result code instead
/// of the regular ERROR final result code. The error result code is returned
/// normally when an error is related to syntax, invalid parameters or MT
/// functionality
#[derive(Clone, AtatCmd)]
#[at_cmd("+CMEE", NoResponse)]
pub struct SetReportMobileTerminationError {
    #[at_arg(position = 0)]
    pub n: TerminationErrorMode,
}

#[derive(Clone, AtatCmd)]
#[at_cmd("+CMEE?", ReportMobileTerminationError)]
pub struct GetReportMobileTerminationError;

/// 5.20 Extended error report +CEER
///
/// Causes the MT to return one or more lines of the information text response
/// which offer an extended report of the reason for:
/// - the failure in the last unsuccessful call setup or in-call modification,
/// - the last call release,
/// - the last unsuccessful GPRS attach / EPS bearer establishment or
///   unsuccessful PDP context activation,
/// - the last GPRS / EPS bearer detach or PDP context deactivation.
///
/// **NOTES:**
///
/// - TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 /
/// LISA-U1 / SARA-G3 / LEON-G1 The total number of characters in the
/// information text response shall not exceed 2041 characters (including line
/// terminators). The textual report is the failure cause according to 3GPP TS
/// 24.008 [69].
/// - SARA-G4 / SARA-G3 The command also provides an extended report about the
/// reason for the last SM STATUS message sent to the network. When <type>="SM
/// STATUS msg sent" is reported, it is suggested to reset the PS data
/// connection.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CEER", ExtendedErrorReport)]
pub struct GetExtendedErrorReport;
