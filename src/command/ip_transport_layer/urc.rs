//! Responses for Internet protocol transport layer Commands
use super::types;
use crate::socket::SocketHandle;
use atat::atat_derive::{AtatResp, AtatUrc};
use atat::{AtatResp, AtatUrc};

#[derive(Clone, AtatResp)]
pub struct SocketDataAvailable {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}
