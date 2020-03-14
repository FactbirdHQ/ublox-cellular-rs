//! Responses for GPIO Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use atat::AtatResp;
use heapless::{consts, String};

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
