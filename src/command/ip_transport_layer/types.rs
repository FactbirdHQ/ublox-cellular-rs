use ufmt::derive::uDebug;
use serde_repr::{Serialize_repr, Deserialize_repr};


#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SocketProtocol {
    TCP = 6,
    UDP = 17,
}
