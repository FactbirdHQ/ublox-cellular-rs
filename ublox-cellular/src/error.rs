use super::ClockError;

use crate::network::Error as NetworkError;
use crate::services::data::Error as DataServiceError;

#[derive(Debug, PartialEq)]
pub enum GenericError {
    Timeout,
    Clock(ClockError),
    Unsupported,
}

impl From<ClockError> for GenericError {
    fn from(e: ClockError) -> Self {
        GenericError::Clock(e)
    }
}

#[derive(Debug, PartialEq)]
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

impl From<ClockError> for Error {
    fn from(e: ClockError) -> Self {
        Error::Generic(GenericError::Clock(e))
    }
}
