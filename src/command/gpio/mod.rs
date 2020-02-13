//! 4 General Commands
pub mod responses;
pub mod types;

use atat::{Error, atat_derive::ATATCmd, ATATCmd};
use heapless::{consts, String};
use responses::*;
use types::*;

use super::NoResponse;




/// 20.2 GPIO select configuration command +UGPIOC
/// Configures the GPIOs pins as input, output or to handle a custom function. When the GPIOs pins are configured
/// as output pin, it is possible to set the value.
/// The test command provides the list of the supported GPIOs, the supported functions and the status of all the
/// GPIOs.
#[derive(Clone, ATATCmd)]
#[at_cmd("+UGPIOC", NoResponse, timeout_ms = 10000)]
pub struct SetGpioConfiguration {
    /// GPIO pin identifier: pin number
    /// See the GPIO mapping for the available GPIO pins, their mapping and factoryprogrammed values on different u-blox cellular modules series and product version.
    //#[atat_(position = 0)]
    pub gpio_id: u8,
    /// Mode identifier: configured function
    /// See the GPIO functions for custom functions supported by different u-blox cellular
    /// modules series and product version
    //#[atat_(position = 1)]
    pub gpio_mode: GpioMode,
}
#[derive(Clone, ATATCmd)]
#[at_cmd("+UGPIOC?", GpioConfiguration, timeout_ms = 10000)]
pub struct GetGpioConfiguration;
