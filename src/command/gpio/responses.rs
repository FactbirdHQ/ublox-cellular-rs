//! Responses for GPIO Commands
use super::types::GpioMode;
use atat::atat_derive::AtatResp;

/// 20.2 GPIO select configuration command +UGPIOC
#[derive(Clone, AtatResp)]
pub struct GpioConfiguration {
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

/// 20.3.3 Defined values
#[derive(Clone, AtatResp)]
pub struct GpioPinValue {
    /// Number GPIO pin identifier: pin number
    #[at_arg(position = 0)]
    pub gpio_id: u8,
    /// Number GPIO value. Allowed values are 0 and 1.
    #[at_arg(position = 1)]
    pub gpio_val: u8,
}
