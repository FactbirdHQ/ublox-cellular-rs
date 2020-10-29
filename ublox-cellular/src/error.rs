// use atat::Error as ATError;
// use heapless::{consts::U64, String};

use crate::socket;

#[derive(Debug, defmt::Format)]
pub enum Error {
    SetState,
    BadLength,
    Network,
    Pin,
    BaudDetection,
    SocketClosed,
    WrongSocketType,
    SocketNotFound,
    NetworkState(crate::State),
    Socket(socket::Error),
    BorrowError,
    BorrowMutError,
    AT(atat::Error),
    Busy,
    BufferFull,
    InvalidHex,
    Dns,
    Uninitialized,
    _Unknown,
}

impl From<atat::Error> for Error {
    fn from(e: atat::Error) -> Self {
        Error::AT(e)
    }
}

impl From<socket::Error> for Error {
    fn from(e: crate::socket::Error) -> Self {
        Error::Socket(e)
    }
}

impl From<core::cell::BorrowMutError> for Error {
    fn from(_: core::cell::BorrowMutError) -> Self {
        Error::BorrowMutError
    }
}

impl From<core::cell::BorrowError> for Error {
    fn from(_: core::cell::BorrowError) -> Self {
        Error::BorrowError
    }
}
