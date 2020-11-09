use crate::APNInfo;
use embedded_hal::digital::{InputPin, OutputPin};
use heapless::{consts, String};

pub struct NoPin;

impl InputPin for NoPin {
    type Error = core::convert::Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl OutputPin for NoPin {
    type Error = core::convert::Infallible;

    fn try_set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn try_set_high(&mut self) -> Result<(), Self::Error> {
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
    pub(crate) apn_info: APNInfo,
    pub(crate) pin: String<consts::U4>,
}

impl<RST, DTR, PWR, VINT> Default for Config<RST, DTR, PWR, VINT> {
    fn default() -> Self {
        Config {
            rst_pin: None,
            dtr_pin: None,
            pwr_pin: None,
            vint_pin: None,
            baud_rate: 115_200_u32,
            hex_mode: true,
            flow_control: false,
            apn_info: APNInfo::default(),
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
    pub fn new(pin: &str) -> Self {
        Config {
            rst_pin: None,
            dtr_pin: None,
            pwr_pin: None,
            vint_pin: None,
            baud_rate: 115_200_u32,
            hex_mode: true,
            flow_control: false,
            apn_info: APNInfo::default(),
            pin: String::from(pin),
        }
    }

    pub fn with_rst(self, rst_pin: RST) -> Self {
        Config {
            rst_pin: Some(rst_pin),
            ..self
        }
    }

    pub fn with_pwr(self, pwr_pin: PWR) -> Self {
        Config {
            pwr_pin: Some(pwr_pin),
            ..self
        }
    }

    pub fn with_dtr(self, dtr_pin: DTR) -> Self {
        Config {
            dtr_pin: Some(dtr_pin),
            ..self
        }
    }

    pub fn with_vint(self, vint_pin: VINT) -> Self {
        Config {
            vint_pin: Some(vint_pin),
            ..self
        }
    }

    pub fn baud_rate<B: Into<u32>>(self, baud_rate: B) -> Self {
        // FIXME: Validate baudrates

        Config {
            baud_rate: baud_rate.into(),
            ..self
        }
    }

    pub fn with_flow_control(self) -> Self {
        Config {
            flow_control: true,
            ..self
        }
    }

    pub fn with_apn_info(self, apn_info: APNInfo) -> Self {
        Config { apn_info, ..self }
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }
}
