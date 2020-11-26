//! Unsolicited responses for Internet protocol transport layer Commands
use crate::services::data::socket::SocketHandle;
use atat::atat_derive::AtatResp;

/// +UUSORD/+UUSORF
#[derive(Clone, AtatResp)]
pub struct SocketDataAvailable {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}

/// +UUSOCL
#[derive(Clone, AtatResp)]
pub struct SocketClosed {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
}
