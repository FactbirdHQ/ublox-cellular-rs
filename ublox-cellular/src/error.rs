use crate::network::Error as NetworkError;
use crate::services::data::Error as DataServiceError;
use core::cell::{BorrowError, BorrowMutError};

#[derive(Debug, defmt::Format)]
pub enum GenericError {
    BorrowError,
    BorrowMutError,
}

impl From<BorrowMutError> for GenericError {
    fn from(_: BorrowMutError) -> Self {
        GenericError::BorrowMutError
    }
}

impl From<BorrowError> for GenericError {
    fn from(_: BorrowError) -> Self {
        GenericError::BorrowError
    }
}

#[derive(Debug, defmt::Format)]
#[non_exhaustive]
pub enum Error {
    // General device errors
    BaudDetection,
    Busy,
    Uninitialized,
    StateTimeout,

    // Network errors
    Network(NetworkError),

    // Service specific errors
    DataService(DataServiceError),

    // Generic shared errors, e.g. from `core::`
    Generic(GenericError),

    _Unknown,
}

impl From<DataServiceError> for Error {
    fn from(e: DataServiceError) -> Self {
        // Unwrap generic and network errors
        match e {
            DataServiceError::Generic(g) => Error::Generic(g),
            DataServiceError::Network(g) => Error::Network(g),
            _ => Error::DataService(e),
        }
    }
}

impl From<NetworkError> for Error {
    fn from(e: NetworkError) -> Self {
        // Unwrap generic errors
        match e {
            NetworkError::Generic(g) => Error::Generic(g),
            _ => Error::Network(e),
        }
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
