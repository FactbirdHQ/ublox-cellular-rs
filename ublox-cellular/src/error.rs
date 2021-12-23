use embedded_time::TimeError;

use crate::command::control::types::BaudRate;
use crate::network::Error as NetworkError;
use crate::services::data::Error as DataServiceError;

#[derive(Debug, PartialEq)]
pub enum GenericError {
    Timeout,
    Time(TimeError),
    Unsupported,
}

impl From<TimeError> for GenericError {
    fn from(e: TimeError) -> Self {
        GenericError::Time(e)
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

    /// The device was reconfigured for a new baud rate.
    /// Caller should reconfigure it's UART interface to this rate.
    NeedsBaudReconnect(BaudRate),

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

impl From<TimeError> for Error {
    fn from(e: TimeError) -> Self {
        Error::Generic(e.into())
    }
}
