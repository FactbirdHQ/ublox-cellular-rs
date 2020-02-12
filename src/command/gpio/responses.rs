//! 4 Responses for General Commands
use heapless::{consts, String};
use serde::Deserialize;



/// 20.2 GPIO select configuration command +UGPIOC
// #[derive(Deserialize)]
pub struct GpioConfiguration{
    /// GPIO pin identifier: pin number
    /// See the GPIO mapping for the available GPIO pins, their mapping and factoryprogrammed values on different u-blox cellular modules series and product version.
    //#[atat_(position = 0)]
    gpio_id: u8,
    /// Mode identifier: configured function
    /// See the GPIO functions for custom functions supported by different u-blox cellular
    /// modules series and product version
    //#[atat_(position = 1)]
    gpio_mode: GpioMode,
}

