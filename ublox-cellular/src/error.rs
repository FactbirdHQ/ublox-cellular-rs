use crate::network::Error as NetworkError;
use crate::services::data::Error as DataServiceError;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GenericError {
    Timeout,
    Clock,
    Unsupported,
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
            DataServiceError::Generic(g) => Self::Generic(g),
            DataServiceError::Network(g) => Self::Network(g),
            _ => Self::DataService(e),
        }
    }
}

impl From<NetworkError> for Error {
    fn from(e: NetworkError) -> Self {
        // Unwrap generic errors
        match e {
            NetworkError::Generic(g) => Self::Generic(g),
            _ => Self::Network(e),
        }
    }
}

// `Clock` trait has associated `Error` type.
// Therefore we cannot use `From` for error converion.
// This is helper that can be used as `.map_err(from_clock)`
pub fn from_clock<E>(_error: E) -> Error {
    Error::Generic(GenericError::Clock)
}
