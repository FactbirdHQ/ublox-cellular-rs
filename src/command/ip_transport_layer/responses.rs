//! 4 Responses for General Commands
use heapless::{consts, String};
use serde::Deserialize;
use crate::socket::SocketHandle;


/// 25.3 Create Socket +USOCR
// #[derive(Deserialize)]
pub struct CreateSocketResponse{
    socket: SocketHandle,
}

/// 25.8 Get Socket Error +USOER
// #[derive(Deserialize)]
pub struct SocketErrorResponse{
    error : u8
}

/// 25.10 Write socket data +USOWR
// #[derive(Deserialize)]
pub struct WriteSocketDataResponse{
    //#[atat_(position = 0)]
    socket: SocketHandle,
    //#[atat_(position = 1)]
    length: usize,
}

/// 25.12 Read Socket Data +USORD
// #[derive(Deserialize)]
pub struct SocketData {
    //#[atat_(position = 0)]
    socket: SocketHandle,
    //#[atat_(position = 1)]
    length: usize,
    //#[atat_(position = 2)]
    data: Vec<u8, consts::U256>
}
