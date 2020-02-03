use super::error::*;

use embedded_hal::timer::{Cancel, CountDown};
use heapless::Vec;

/// Wireless network connectivity functionality.
pub trait DataConnectivity<T>
where
    T: CountDown + Cancel,
    T::Time: Copy,
{
    // Makes an attempt to connect to a selected wireless network with password specified.
    // fn connect(self) -> Result<WifiConnection<T>, WifiConnectionError>;

    // fn scan(&mut self) -> Result<Vec<WifiNetwork, at::MaxResponseLines>, WifiError>;
}
