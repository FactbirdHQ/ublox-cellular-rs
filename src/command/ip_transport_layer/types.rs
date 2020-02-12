use serde::{Serialize, Deserialize};
use ufmt::derive::uDebug;


#[derive(uDebug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SocketProtocol {
    TCP = 6,
    UDP = 17,
}
