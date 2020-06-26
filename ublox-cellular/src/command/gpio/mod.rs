//! ### 20 - GPIO Commands
//! The section describes the AT commands used to configure the GPIO pins provided by u-blox cellular modules
//! ### GPIO functions
//! On u-blox cellular modules, GPIO pins can be opportunely configured as general purpose input or output.
//! Moreover GPIO pins of u-blox cellular modules can be configured to provide custom functions via +UGPIOC
//! AT command. The custom functions availability can vary depending on the u-blox cellular modules series and
//! version: see Table 53 for an overview of the custom functions supported by u-blox cellular modules. \
//! The configuration of the GPIO pins (i.e. the setting of the parameters of the +UGPIOC AT command) is saved
//! in the NVM and used at the next power-on.
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use responses::*;
use types::*;

use super::NoResponse;

/// 20.2 Set GPIO select configuration command +UGPIOC
///
/// Configures the GPIOs pins as input, output or to handle a custom function. When the GPIOs pins are configured
/// as output pin, it is possible to set the value.
/// The test command provides the list of the supported GPIOs, the supported functions and the status of all the
/// GPIOs.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UGPIOC", NoResponse)]
pub struct SetGpioConfiguration {
    /// GPIO pin identifier: pin number
    /// See the GPIO mapping for the available GPIO pins, their mapping and factoryprogrammed values on different u-blox cellular modules series and product version.
    #[at_arg(position = 0)]
    pub gpio_id: u8,
    /// Mode identifier: configured function
    /// See the GPIO functions for custom functions supported by different u-blox cellular
    /// modules series and product version
    #[at_arg(position = 1)]
    pub gpio_mode: GpioMode,
}

/// 20.2 Get GPIO select configuration command +UGPIOC
///
/// Configures the GPIOs pins as input, output or to handle a custom function. When the GPIOs pins are configured
/// as output pin, it is possible to set the value.
/// The test command provides the list of the supported GPIOs, the supported functions and the status of all the
/// GPIOs.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UGPIOC?", GpioConfiguration)]
pub struct GetGpioConfiguration;
