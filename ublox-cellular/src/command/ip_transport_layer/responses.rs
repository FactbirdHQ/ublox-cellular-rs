//! Responses for Internet protocol transport layer Commands
use super::types::SocketControlParam;
use crate::services::data::INGRESS_CHUNK_SIZE;
use atat::atat_derive::AtatResp;
use embedded_nal::IpAddr;
use heapless::String;
use ublox_sockets::SocketHandle;

/// 25.3 Create Socket +USOCR
#[derive(Debug, Clone, AtatResp)]
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
    // Note: Data max length is `INGRESS_CHUNK_SIZE` * 2, due to hex encoding
    pub data: Option<String<{ INGRESS_CHUNK_SIZE * 2 }>>,
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
    // Note: Data max length is `INGRESS_CHUNK_SIZE` * 2, due to hex encoding
    pub data: Option<String<{ INGRESS_CHUNK_SIZE * 2 }>>,
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
