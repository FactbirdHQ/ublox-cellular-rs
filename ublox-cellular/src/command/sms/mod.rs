//! ### 11 - Short Messages Service

pub mod responses;
pub mod types;
pub mod urc;

use super::NoResponse;
use atat::atat_derive::AtatCmd;
use types::*;

/// 11.29 Message waiting indication +UMWI
///
/// Provides information regarding the Message Waiting Indication (MWI) third level method (3GPP defined in
/// 3GPP TS 23.040 [8]) and CPHS method [54] following AT&T Device Requirements [49].
///
/// The set command enables / disables the URC presentation. The URCs are by default enabled.
///
/// MWI is based on specific EFs not present in all SIM cards. In case these EFs are not present, the information
/// text response is an error result code ("+CME ERROR: operation not allowed" if +CMEE is set to 2) and no URCs
/// will be displayed.
///
/// The URCs are displayed in groups of variable number which depends on the EFs present in the SIM card
/// 3GPP TS 31.102 [18] and Common PCN Handset Specification
#[derive(Clone, AtatCmd)]
#[at_cmd("+UMWI", NoResponse)]
pub struct SetMessageWaitingIndication {
    #[at_arg(position = 0)]
    pub mode: MessageWaitingMode,
}
