use crate::error::GenericError;
use crate::network::Error as NetworkError;
use crate::ClockError;
use ublox_sockets::Error as SocketError;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidApn,
    SocketClosed,
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

impl From<ClockError> for Error {
    fn from(e: ClockError) -> Self {
        Error::Generic(GenericError::Clock(e))
    }
}
