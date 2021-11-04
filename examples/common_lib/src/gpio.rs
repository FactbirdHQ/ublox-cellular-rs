use embedded_hal::digital::{InputPin, OutputPin};
use linux_embedded_hal::{sysfs_gpio, Pin};

// implement newest embedded_hal traits
// linux_embedded_hal uses old ones
pub struct ExtPin(Pin);

impl OutputPin for ExtPin {
    type Error = sysfs_gpio::Error;

    fn try_set_low(&mut self) -> Result<(), Self::Error> {
        if self.0.get_active_low()? {
            self.0.set_value(1)
        } else {
            self.0.set_value(0)
        }
    }

    fn try_set_high(&mut self) -> Result<(), Self::Error> {
        if self.0.get_active_low()? {
            self.0.set_value(0)
        } else {
            self.0.set_value(1)
        }
    }
}

impl InputPin for ExtPin {
    type Error = sysfs_gpio::Error;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        if !self.0.get_active_low()? {
            self.0.get_value().map(|val| val != 0)
        } else {
            self.0.get_value().map(|val| val == 0)
        }
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        self.try_is_high().map(|val| !val)
    }
}
