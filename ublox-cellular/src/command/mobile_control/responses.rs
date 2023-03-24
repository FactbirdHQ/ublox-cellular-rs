//! Responses for Mobile equipment control and status Commands
use super::types::{PowerMode, ReportMobileTerminationErrorStatus, STKMode};
use atat::atat_derive::AtatResp;

/// 5.3 Set module functionality +CFUN
/// Selects the level of functionality <fun> in the MT.
#[derive(Clone, AtatResp)]
pub struct ModuleFunctionality {
    #[at_arg(position = 0)]
    pub power_mode: PowerMode,
    #[at_arg(position = 1)]
    pub stk_mode: Option<STKMode>,
}

/// 5.4 Indicator control +CIND
///
/// Provides indication states related to network status, battery information and so on.
/// The set command does not allow setting the values for those indications which are set according to module
/// state (see <descr> parameter).
/// The list of indications for set and read commands follows the indexes reported in the <descr> parameter, so
/// that the first <ind> corresponds to "battchg" and so on
#[derive(Clone, Debug, AtatResp)]
pub struct IndicatorControl {
    /// "battchg": battery charge level (0-5)
    #[at_arg(position = 0)]
    pub battchg: u8,
    /// "signal": signal level. See mapping in the Notes below
    #[at_arg(position = 1)]
    pub signal: u8,
    /// "service": network service availability
    /// o 0: not registered to any network
    /// o 1: registered to the network
    /// o 65535: indication not available
    #[at_arg(position = 2)]
    pub service: u16,
    /// • "sounder": sounder activity, indicating when the module is generating a sound
    /// o 0: no sound
    /// o 1: sound is generated
    #[at_arg(position = 3)]
    pub sounder: u8,
    /// • "message": unread message available in <mem1> storage
    /// o 0: no messages
    /// o 1: unread message available
    #[at_arg(position = 4)]
    pub message: u8,
    /// • "call": call in progress
    /// o 0: no call in progress
    /// o 1: call in progress
    #[at_arg(position = 5)]
    pub call: u8,
    /// • "roam": registration on a roaming network
    /// o 0: not in roaming or not registered
    /// o 1: roaming
    /// o 65535: indication not available
    #[at_arg(position = 6)]
    pub roam: u16,
    /// • "smsfull": indication that an SMS has been rejected with the cause of SMS storage
    /// full
    /// o 0: SMS storage not full
    /// o 1: SMS storage full
    #[at_arg(position = 7)]
    pub smsfull: u8,
    /// • "gprs": PS indication status:
    /// o 0: no PS available in the network
    /// o 1: PS available in the network but not registered
    /// o 2: registered to PS
    /// o 65535: indication not available
    #[at_arg(position = 8)]
    pub gprs: u16,
    /// • "callsetup": call set-up:
    /// o 0: no call set-up
    /// o 1: incoming call not accepted or rejected
    /// o 2: outgoing call in dialling state
    /// o 3: outgoing call in remote party alerting state
    #[at_arg(position = 9)]
    pub callsetup: u8,
    /// • "callheld": call on hold:
    /// o 0: no calls on hold
    /// o 1: at least one call on hold
    #[at_arg(position = 10)]
    pub callheld: u8,
    /// • "simind": SIM detection
    /// o 0: no SIM detected
    /// o 1: SIM detected
    /// o 2: not available
    #[at_arg(position = 11)]
    pub simind: u8,
}

/// 5.7 Clock +CCLK
///
/// Reads the real-time clock of the MT
#[derive(Clone, Debug, AtatResp)]
pub struct DateTime {
    pub time: heapless::String<20>,
}

/// 5.19 Report mobile termination error +CMEE
///
/// Configures the formatting of the result code +CME ERROR: <err> as an indication of an error relating to the
/// functionality of the MT. When enabled, MT related errors cause +CME ERROR: <err> final result code instead
/// of the regular ERROR final result code. The error result code is returned normally when an error is related to
/// syntax, invalid parameters or MT functionality
#[derive(Clone, AtatResp)]
pub struct ReportMobileTerminationError {
    #[at_arg(position = 0)]
    pub status: ReportMobileTerminationErrorStatus,
}

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
#[derive(Clone, AtatResp)]
pub struct ExtendedErrorReport {
    #[at_arg(position = 0)]
    pub r#type: heapless::String<32>,
    #[at_arg(position = 1)]
    pub cause: u32,
    #[at_arg(position = 2)]
    pub description: heapless::String<64>,
}
