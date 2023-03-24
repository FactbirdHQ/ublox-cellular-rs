use embedded_hal::digital::{ErrorType, InputPin, OutputPin};
use heapless::String;

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

#[derive(Debug)]
pub struct Config<RST, DTR, PWR, VINT> {
    pub(crate) rst_pin: Option<RST>,
    pub(crate) dtr_pin: Option<DTR>,
    pub(crate) pwr_pin: Option<PWR>,
    pub(crate) vint_pin: Option<VINT>,
    pub(crate) baud_rate: u32,
    pub(crate) hex_mode: bool,
    pub(crate) flow_control: bool,
    pub(crate) pin: String<4>,
}

impl Default for Config<NoPin, NoPin, NoPin, NoPin> {
    fn default() -> Self {
        Self {
            rst_pin: None,
            dtr_pin: None,
            pwr_pin: None,
            vint_pin: None,
            baud_rate: 115_200_u32,
            hex_mode: true,
            flow_control: false,
            pin: String::new(),
        }
    }
}

impl<RST, DTR, PWR, VINT> Config<RST, DTR, PWR, VINT>
where
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
    #[must_use] pub fn new(pin: &str) -> Self {
        Self {
            rst_pin: None,
            dtr_pin: None,
            pwr_pin: None,
            vint_pin: None,
            baud_rate: 115_200_u32,
            hex_mode: true,
            flow_control: false,
            pin: String::from(pin),
        }
    }

    pub fn with_rst(self, rst_pin: RST) -> Self {
        Self {
            rst_pin: Some(rst_pin),
            ..self
        }
    }

    pub fn with_pwr(self, pwr_pin: PWR) -> Self {
        Self {
            pwr_pin: Some(pwr_pin),
            ..self
        }
    }

    pub fn with_dtr(self, dtr_pin: DTR) -> Self {
        Self {
            dtr_pin: Some(dtr_pin),
            ..self
        }
    }

    pub fn with_vint(self, vint_pin: VINT) -> Self {
        Self {
            vint_pin: Some(vint_pin),
            ..self
        }
    }

    pub fn baud_rate<B: Into<u32>>(self, baud_rate: B) -> Self {
        // FIXME: Validate baudrates

        Self {
            baud_rate: baud_rate.into(),
            ..self
        }
    }

    pub fn with_flow_control(self) -> Self {
        Self {
            flow_control: true,
            ..self
        }
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }
}
