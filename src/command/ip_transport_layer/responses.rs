//! 4 Responses for General Commands
use heapless::{consts, Vec};
use atat::atat_derive::ATATResp;
use atat::ATATResp;
use super::types::*;
use crate::socket::SocketHandle;


/// 25.3 Create Socket +USOCR
#[derive(Clone, ATATResp)]
pub struct CreateSocketResponse{
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
}

/// 25.8 Get Socket Error +USOER
#[derive(Clone, ATATResp)]
pub struct SocketErrorResponse{
    #[at_arg(position = 0)]
    pub error : u8
}

/// 25.10 Write socket data +USOWR
#[derive(Clone, ATATResp)]
pub struct WriteSocketDataResponse{
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}

/// 25.12 Read Socket Data +USORD
#[derive(Clone, ATATResp)]
pub struct SocketData {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
    #[at_arg(position = 2)]
    pub data: Vec<u8, consts::U256>
}
