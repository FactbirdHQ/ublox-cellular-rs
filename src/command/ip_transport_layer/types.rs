//! Argument and parameter types used by Internet protocol transport layer Commands and Responses
use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SocketProtocol {
    TCP = 6,
    UDP = 17,
}
