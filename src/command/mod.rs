//! AT Commands for U-Blox short range module family\
//! Following the [u-connect ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

pub mod general;
pub mod mobile_control;
pub mod network_service;
pub mod device_lock;
pub mod sms;
pub mod control;
pub mod device_data_security;
pub mod gpio;
pub mod ip_transport_layer;
pub mod psn;
pub mod system_features;

use atat::atat_derive::{AtatCmd, AtatResp, AtatUrc};

#[derive(Clone, AtatResp)]
pub struct NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 1000)]
pub struct AT;

#[derive(Clone, AtatUrc)]
pub enum Urc {
    #[at_urc("+UUSORD")]
    SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
    #[at_urc("+UUPSDD")]
    DataConnectionDeactivated(psn::urc::DataConnectionDeactivated),
    #[at_urc("+UUSOCL")]
    SocketClosed(ip_transport_layer::urc::SocketClosed),
    #[at_urc("+UMWI")]
    MessageWaitingIndication(sms::urc::MessageWaitingIndication),
}
