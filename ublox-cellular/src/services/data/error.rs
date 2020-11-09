use core::cell::{BorrowError, BorrowMutError};
use crate::network::Error as NetworkError;
use crate::error::GenericError;
use super::socket::Error as SocketError;

#[derive(Debug, defmt::Format)]
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

    _Unknown
}

impl From<NetworkError> for Error {
    fn from(e: NetworkError) -> Self {
        match e {
            NetworkError::Generic(g) => Error::Generic(g),
            _ => Error::Network(e)
        }
    }
}

impl From<SocketError> for Error {
    fn from(e: SocketError) -> Self {
        Error::Socket(e)
    }
}

impl From<BorrowMutError> for Error {
    fn from(e: BorrowMutError) -> Self {
        Error::Generic(e.into())
    }
}

impl From<BorrowError> for Error {
    fn from(e: BorrowError) -> Self {
        Error::Generic(e.into())
    }
}
