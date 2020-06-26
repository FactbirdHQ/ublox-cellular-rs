//! AT Commands for u-blox cellular module family\
//! Following the [u-blox cellular modules AT commands manual](https://www.u-blox.com/sites/default/files/u-blox-CEL_ATCommands_%28UBX-13002752%29.pdf)

pub mod control;
pub mod device_data_security;
pub mod device_lock;
pub mod dns;
pub mod general;
pub mod gpio;
pub mod ip_transport_layer;
pub mod mobile_control;
pub mod network_service;
pub mod psn;
pub mod sms;
pub mod system_features;

use atat::atat_derive::{AtatCmd, AtatResp, AtatUrc};

#[derive(Clone, AtatResp)]
pub struct NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse)]
pub struct AT;

#[derive(Clone, AtatUrc)]
pub enum Urc {
    #[at_urc("+UUSORD")]
    SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
    #[at_urc("+UUPSDA")]
    DataConnectionActivated(psn::urc::DataConnectionActivated),
    #[at_urc("+UUPSDD")]
    DataConnectionDeactivated(psn::urc::DataConnectionDeactivated),
    #[at_urc("+UUSOCL")]
    SocketClosed(ip_transport_layer::urc::SocketClosed),
    #[at_urc("+UMWI")]
    MessageWaitingIndication(sms::urc::MessageWaitingIndication),
}

impl atat::AtatLen for crate::sockets::SocketHandle {
    type Len = heapless::consts::U1;
}
