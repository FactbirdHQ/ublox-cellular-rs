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

#[cfg(feature = "defmt")]
impl defmt::Format for Error {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self {
            Self::BaudDetection => defmt::write!(f, "BaudDetection"),
            Self::Busy => defmt::write!(f, "Busy"),
            Self::Uninitialized => defmt::write!(f, "Uninitialized"),
            Self::StateTimeout => defmt::write!(f, "StateTimeout"),
            Self::Network(e) => defmt::write!(f, "Network({:?})", e),
            Self::DataService(e) => defmt::write!(f, "DataService({:?})", e),
            Self::Generic(e) => defmt::write!(f, "Generic({:?})", e),
            Self::_Unknown => defmt::write!(f, "_Unknown"),
            _ => defmt::write!(f, "non_exhaustive"),
        }
    }
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
