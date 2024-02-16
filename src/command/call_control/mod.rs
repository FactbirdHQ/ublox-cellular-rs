//! ### 6 - Call control
mod types;

use atat::atat_derive::AtatCmd;

use self::types::AddressType;

use super::NoResponse;

/// 6.1 Select type of address +CSTA
///
/// Selects the type of number for further dialling commands (D) according to
/// 3GPP specifications.
///
/// **NOTES:**
/// - The type of address is automatically detected from the dialling string
///   thus the +CSTA command has no effect.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CSTA", NoResponse)]
pub struct SetAddressType {
    #[at_arg(position = 0)]
    pub typ: AddressType,
}

/// 6.2 Dial command D
///
/// Lists characters that may be used in a dialling string for making a call
/// (voice, data or fax call) or controlling supplementary services in
/// accordance with 3GPP TS 22.030 [77] and initiates the indicated kind of
/// call. No further commands may follow in the command line in case of data or
/// fax calls.
///
/// **NOTES:**
/// - **LARA-L6 / LARA-R6**: Supplementary services strings are not supported in
///   the dial command. Set the DTR line to ON state before making a data call
/// - **LARA-L6004D / LARA-R6001D / LARA-R6401D**: Voice calls are not
///   supported.
#[derive(Clone, AtatCmd)]
#[at_cmd(
    "D",
    NoResponse,
    abortable = true,
    timeout_ms = 180000,
    value_sep = false
)]
pub struct Dial<'a> {
    /// Dial string; the allowed characters are: 1 2 3 4 5 6 7 8 9 0 * # + A B C
    /// D , T P ! W @ (see the 3GPP TS 27.007 [75]). The following characters
    /// are ignored: , T ! W @.
    ///
    /// **NOTE**: The first occurrence of P is interpreted as pause and
    /// separator between the dialling number and the DTMF string. The following
    /// occurrences are interpreted only as pause. The use of P as pause has
    /// been introduced for AT&T certification.
    #[at_arg(position = 0, len = 32)]
    pub number: &'a str,
}
