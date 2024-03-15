use core::convert::Infallible;
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, PinState};

use crate::command::psn::types::{ContextId, ProfileId};

pub struct NoPin;

impl ErrorType for NoPin {
    type Error = core::convert::Infallible;
}

impl InputPin for NoPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
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

pub struct ReverseOutputPin<P: OutputPin<Error = Infallible>>(pub P);

impl<P: OutputPin<Error = Infallible>> ErrorType for ReverseOutputPin<P> {
    type Error = Infallible;
}

impl<P: OutputPin<Error = Infallible>> OutputPin for ReverseOutputPin<P> {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_high()
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_low()
    }

    fn set_state(&mut self, state: PinState) -> Result<(), Self::Error> {
        match state {
            PinState::Low => self.0.set_state(PinState::High),
            PinState::High => self.0.set_state(PinState::Low),
        }
    }
}

pub struct ReverseInputPin<P: InputPin<Error = Infallible>>(pub P);

impl<P: InputPin<Error = Infallible>> ErrorType for ReverseInputPin<P> {
    type Error = Infallible;
}

impl<P: InputPin<Error = Infallible>> InputPin for ReverseInputPin<P> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.0.is_low()
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.0.is_high()
    }
}

pub trait CellularConfig<'a> {
    type ResetPin: OutputPin;
    type PowerPin: OutputPin;
    type VintPin: InputPin;

    const FLOW_CONTROL: bool = false;

    #[cfg(feature = "internal-network-stack")]
    const HEX_MODE: bool = true;

    const OPERATOR_FORMAT: OperatorFormat = OperatorFormat::Long;

    const PROFILE_ID: ProfileId = ProfileId(1);
    const CONTEXT_ID: ContextId = ContextId(1);

    const APN: Apn<'a> = Apn::None;

    #[cfg(feature = "ppp")]
    const PPP_CONFIG: embassy_net_ppp::Config<'a>;

    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        None
    }
    fn power_pin(&mut self) -> Option<&mut Self::PowerPin> {
        None
    }
    fn vint_pin(&mut self) -> Option<&mut Self::VintPin> {
        None
    }
}

#[repr(u8)]
pub enum OperatorFormat {
    Long = 0,
    Short = 1,
    Numeric = 2,
}

#[derive(Debug, Clone)]
pub enum Apn<'a> {
    None,
    Given {
        name: &'a str,
        username: Option<&'a str>,
        password: Option<&'a str>,
    },
    #[cfg(any(feature = "automatic-apn"))]
    Automatic,
}

impl Default for Apn<'_> {
    fn default() -> Self {
        Self::None
    }
}
