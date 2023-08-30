use embedded_hal::digital::{ErrorType, InputPin, OutputPin};

pub struct NoPin;

impl ErrorType for NoPin {
    type Error = core::convert::Infallible;
}

impl InputPin for NoPin {
    fn is_high(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn is_low(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }
}

impl OutputPin for NoPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    /// No flow control is being used
    None,
    /// Hardware flow control
    RtsCts,
}

pub trait CellularConfig {
    type ResetPin: OutputPin;
    type PowerPin: OutputPin;
    type VintPin: InputPin;

    const FLOW_CONTROL: FlowControl = FlowControl::None;
    const HEX_MODE: bool = true;

    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin>;
    fn power_pin(&mut self) -> Option<&mut Self::PowerPin>;
    fn vint_pin(&mut self) -> Option<&mut Self::VintPin>;
}
