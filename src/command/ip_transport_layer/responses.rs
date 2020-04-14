//! Responses for Internet protocol transport layer Commands
use super::types::*;
use crate::socket::SocketHandle;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};
use no_std_net::IpAddr;

/// 25.3 Create Socket +USOCR
#[derive(Clone, AtatResp)]
pub struct CreateSocketResponse {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
}

/// 25.8 Get Socket Error +USOER
#[derive(Clone, AtatResp)]
pub struct SocketErrorResponse {
    #[at_arg(position = 0)]
    pub error: u8,
}

/// 25.10 Write socket data +USOWR
#[derive(Clone, AtatResp)]
pub struct WriteSocketDataResponse {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}

/// 25.11 UDP Send To data +USOST:
#[derive(Clone, AtatResp)]
pub struct UDPSendToDataResponse {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}

/// 25.12 Read Socket Data +USORD
#[derive(Clone, AtatResp)]
pub struct SocketData {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
    #[at_arg(position = 2)]
    pub data: String<consts::U256>,
}

/// 25.13 Read UDP Socket Data +USORF
#[derive(Clone, AtatResp)]
pub struct UDPSocketData {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub remote_addr: IpAddr,
    #[at_arg(position = 2)]
    pub remote_port: u16,
    #[at_arg(position = 3)]
    pub length: usize,
    #[at_arg(position = 4)]
    pub data: String<consts::U256>,
}

/// 25.25 Socket control +USOCTL
#[derive(Clone, AtatResp)]
pub struct SocketControlResponse {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub param_id: SocketControlParam,
    #[at_arg(position = 2)]
    pub param_val: u32,
}
