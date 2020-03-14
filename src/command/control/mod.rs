//! ### 15 - V24 control and V25ter
//! These commands, unless specifically stated, do not implement set syntax using "=", read ("?"), or test ("=?").
//! If such commands are used, the "+CME ERROR: unknown" or "+CME ERROR: 100" error result code is provided
//! (depending on the +CMEE AT command setting).
// pub mod responses;
pub mod types;

use atat::{atat_derive::AtatCmd, AtatCmd, Error};
use heapless::{consts, String};
// use responses::*;
use types::*;

use super::NoResponse;

/// 15.5 Flow control &K
///
/// Controls the flow control mechanism. The following settings are allowed:
/// • No flow control
/// • HW flow control also referred with RTS / CTS flow control
/// • SW flow control also referred with XON / XOFF flow control
#[derive(Clone, AtatCmd)]
#[at_cmd("&K", NoResponse, value_sep = false)]
pub struct SetFlowControl {
    #[at_arg(position = 0)]
    pub value: FlowControl,
}

/// 15.8 Set flow control \Q
///
/// Controls the operation of the local flow control between DTE and DCE. It is used when the data are sent or
/// received.
/// When the software flow control (XON/XOFF) is used, the DC1 (XON, 0x11) and DC3 (XOFF, 0x13) characters are
/// reserved and therefore filtered (e.g. in SMS text mode these two characters can not be input).
/// Since the DTE-DCE communication relies on the correct reception of DC1/DC3 characters, the UART power
/// saving should be disabled on the module when SW flow control is used. If the UART power saving is active, the
/// DC1/DC3 characters could be used to wake up the module's UART, and therefore lost. In case a DC3 character
/// (XOFF) is correctly received by module's UART and some data is waiting to be transmitted, the module is forced
/// to stay awake until a subsequent DC1 character (XON) is received.
#[derive(Clone, AtatCmd)]
#[at_cmd("\\Q", NoResponse, value_sep = false)]
pub struct SetSoftwareFlowControl {
    #[at_arg(position = 0)]
    pub value: SoftwareFlowControl,
}

/// 15.9 UART data rate configuration +IPR
///
/// Specifies the data rate at which the DCE accepts commands on the UART interface. The full range of data
/// rates depends on HW or other criteria.
#[derive(Clone, AtatCmd)]
#[at_cmd("+IPR", NoResponse)]
pub struct SetDataRate {
    #[at_arg(position = 0)]
    pub rate: BaudRate,
}

/// 15.25 Set to factory defined configuration &F
///
/// Resets the current profile to factory-programmed setting. Other NVM settings, not included in the profiles,
/// are not affected.
/// In case of success, the response is issued using the configuration of the result codes format (Q, V, S3, S4
/// AT commands) loaded from the factory-programmed profile. The other DCE settings are applied after the
/// response has been sent.
#[derive(Clone, AtatCmd)]
#[at_cmd("&F", NoResponse)]
pub struct FactoryResetConfig;
