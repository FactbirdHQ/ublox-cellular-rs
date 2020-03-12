//! AT Commands for U-Blox short range module family\
//! Following the [u-connect ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

pub mod control;
pub mod device_lock;
pub mod general;
pub mod gpio;
pub mod ip_transport_layer;
pub mod mobile_control;
pub mod network_service;
pub mod psn;
pub mod sms;
pub mod system_features;

use atat::{atat_derive::ATATUrc, ATATUrc};

use atat::{
    atat_derive::{ATATCmd, ATATResp},
    ATATCmd, ATATResp,
};
use heapless::String;

#[derive(Clone, ATATResp)]
pub struct NoResponse;

#[derive(Clone, ATATCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct AT;

#[derive(Clone, ATATUrc)]
pub enum Urc {
    #[at_urc("+UUSORD")]
    SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
    #[at_urc("+UMWI")]
    MessageWaitingIndication(sms::urc::MessageWaitingIndication),
}
