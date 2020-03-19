//! ### 9 - Device lock

pub mod impl_;
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use responses::*;

use super::NoResponse;

/// 9.1 Enter PIN +CPIN
///
/// Enter PIN. If no PIN request is pending, the corresponding error code is returned. If a wrong PIN is given three
/// times, the PUK must be inserted in place of the PIN, followed by the <newpin> which replaces the old pin in
/// the SIM.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CPIN?", PinStatus, timeout_ms = 10000)]
pub struct GetPinStatus;

/// 9.1 Enter PIN +CPIN
///
/// Enter PIN. If no PIN request is pending, the corresponding error code is returned. If a wrong PIN is given three
/// times, the PUK must be inserted in place of the PIN, followed by the <newpin> which replaces the old pin in
/// the SIM.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CPIN", NoResponse, timeout_ms = 10000)]
pub struct SetPin<'a> {
    #[at_arg(position = 0)]
    pub pin: &'a str,
}

/// 9.1 Enter PIN +CPIN
///
/// Enter PIN. If no PIN request is pending, the corresponding error code is returned. If a wrong PIN is given three
/// times, the PUK must be inserted in place of the PIN, followed by the <newpin> which replaces the old pin in
/// the SIM.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CPIN", NoResponse, timeout_ms = 10000)]
pub struct ChangePin<'a> {
    #[at_arg(position = 0)]
    pub puk: &'a str,
    #[at_arg(position = 1)]
    pub newpin: &'a str,
}
