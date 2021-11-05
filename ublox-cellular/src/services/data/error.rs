use embedded_time::TimeError;

use crate::error::GenericError;
use crate::network::Error as NetworkError;
use ublox_sockets::Error as SocketError;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidApn,
    SocketMemory,
    WrongSocketType,
    BadLength,
    Dns,
    BufferFull,
    InvalidHex,

    Socket(SocketError),

    Network(NetworkError),

    Generic(GenericError),

    _Unknown,
}

impl From<NetworkError> for Error {
    fn from(e: NetworkError) -> Self {
        match e {
            NetworkError::Generic(g) => Error::Generic(g),
            _ => Error::Network(e),
        }
    }
}

impl From<SocketError> for Error {
    fn from(e: SocketError) -> Self {
        Error::Socket(e)
    }
}

impl From<TimeError> for Error {
    fn from(e: TimeError) -> Self {
        Error::Generic(e.into())
    }
}
