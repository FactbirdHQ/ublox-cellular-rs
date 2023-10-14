use crate::error::GenericError;
use crate::network::Error as NetworkError;
use ublox_sockets::Error as SocketError;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
            NetworkError::Generic(g) => Self::Generic(g),
            _ => Self::Network(e),
        }
    }
}

impl From<SocketError> for Error {
    fn from(e: SocketError) -> Self {
        Self::Socket(e)
    }
}
