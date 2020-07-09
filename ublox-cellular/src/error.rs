// use atat::Error as ATError;
// use heapless::{consts::U64, String};

use crate::socket;

#[derive(Debug)]
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
    BorrowError(core::cell::BorrowError),
    BorrowMutError(core::cell::BorrowMutError),
    AT(atat::Error),
    Busy,
    InvalidHex,
    Dns,
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
    fn from(e: core::cell::BorrowMutError) -> Self {
        Error::BorrowMutError(e)
    }
}

impl From<core::cell::BorrowError> for Error {
    fn from(e: core::cell::BorrowError) -> Self {
        Error::BorrowError(e)
    }
}
