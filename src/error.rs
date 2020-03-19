// use atat::Error as ATError;
// use heapless::{consts::U64, String};

use crate::socket;
use atat;

#[derive(Debug)]
pub enum Error {
    BadLength,
    Network,
    Pin,
    BaudDetection,
    SocketClosed,
    WrongSocketType,
    SocketNotFound,
    Socket(socket::Error),
    BorrowError(core::cell::BorrowError),
    BorrowMutError(core::cell::BorrowMutError),
    AT(atat::Error),

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
