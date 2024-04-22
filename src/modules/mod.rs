#[cfg(any(feature = "any-module", feature = "lara-r6"))]
pub(crate) mod lara_r6;
#[cfg(any(feature = "any-module", feature = "lena-r8"))]
pub(crate) mod lena_r8;
#[cfg(any(feature = "any-module", feature = "sara-r410m"))]
pub(crate) mod sara_r410m;
#[cfg(any(feature = "any-module", feature = "sara-r412m"))]
pub(crate) mod sara_r412m;
#[cfg(any(feature = "any-module", feature = "sara-r422"))]
pub(crate) mod sara_r422;
#[cfg(any(feature = "any-module", feature = "sara-r5"))]
pub(crate) mod sara_r5;
#[cfg(any(feature = "any-module", feature = "sara-u201"))]
pub(crate) mod sara_u201;
#[cfg(any(feature = "any-module", feature = "toby-r2"))]
pub(crate) mod toby_r2;

use crate::command::{general::responses::ModelId, mobile_control::types::Functionality};
use embassy_time::Duration;

pub trait ModuleParams: Copy {
    /// The time for which PWR_ON must be pulled down to effect power-on
    fn power_on_pull_time(&self) -> Option<Duration> {
        None
    }

    /// The time for which PWR_ON must be pulled down to effect power-off
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(3100)
    }

    /// How long to wait before the module is ready after boot
    fn boot_wait(&self) -> Duration {
        Duration::from_secs(5)
    }

    /// How long to wait for a organised power-down in the ansence of VInt
    fn power_down_wait(&self) -> Duration {
        Duration::from_secs(35)
    }

    /// How long to wait before the module is ready after it has been commanded
    /// to reboot
    fn reboot_command_wait(&self) -> Duration {
        Duration::from_secs(5)
    }

    /// How long to wait between the end of one AT command and the start of the
    /// next, default value
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(100)
    }

    /// The type of AT+CFUN state to use to switch the radio off: either 0 for
    /// truly off or 4 for "airplane" mode
    fn radio_off_cfun(&self) -> Functionality {
        Functionality::AirplaneMode
    }

    /// How long the reset line has to be held for to reset the cellular module
    fn reset_hold(&self) -> Duration {
        Duration::from_millis(16500)
    }

    /// The maximum number of simultaneous RATs that are supported by the
    /// cellular module
    fn max_num_simultaneous_rats(&self) -> u8 {
        1
    }

    /// Normally 15, but in some cases 16
    fn at_c_fun_reboot_command(&self) -> Functionality {
        Functionality::SilentReset
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Module {
    #[cfg(any(feature = "any-module", feature = "lara-r6"))]
    LaraR6(lara_r6::LaraR6),
    #[cfg(any(feature = "any-module", feature = "lena-r8"))]
    LenaR8(lena_r8::LenaR8),
    #[cfg(any(feature = "any-module", feature = "sara-r410m"))]
    SaraR410m(sara_r410m::SaraR410m),
    #[cfg(any(feature = "any-module", feature = "sara-r412m"))]
    SaraR412m(sara_r412m::SaraR412m),
    #[cfg(any(feature = "any-module", feature = "sara-r422"))]
    SaraR422(sara_r422::SaraR422),
    #[cfg(any(feature = "any-module", feature = "sara-r5"))]
    SaraR5(sara_r5::SaraR5),
    #[cfg(any(feature = "any-module", feature = "sara-u201"))]
    SaraU201(sara_u201::SaraU201),
    #[cfg(any(feature = "any-module", feature = "toby-r2"))]
    TobyR2(toby_r2::TobyR2),
    Generic(Generic),
}

impl Module {
    pub fn from_model_id(model_id: ModelId) -> Self {
        match model_id.model.as_slice() {
            b"LARA-R6001D" => Self::LaraR6(lara_r6::LaraR6),
            id => {
                warn!("Attempting to run {:?} using generic module parameters! This may or may not work.", id);
                Self::Generic(Generic)
            }
        }
    }
}

macro_rules! inner {
    ($self: ident, $fn: ident) => {
        match $self {
            #[cfg(any(feature = "any-module", feature = "lara-r6"))]
            Self::LaraR6(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "lena-r8"))]
            Self::LenaR8(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "sara-r410m"))]
            Self::SaraR410m(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "sara-r412m"))]
            Self::SaraR412m(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "sara-r422"))]
            Self::SaraR422(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "sara-r5"))]
            Self::SaraR5(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "sara-u201"))]
            Self::SaraU201(inner) => inner.$fn(),
            #[cfg(any(feature = "any-module", feature = "toby-r2"))]
            Self::TobyR2(inner) => inner.$fn(),
            Self::Generic(inner) => inner.$fn(),
        }
    };
}

impl ModuleParams for Module {
    fn power_on_pull_time(&self) -> Option<Duration> {
        inner!(self, power_on_pull_time)
    }

    fn power_off_pull_time(&self) -> Duration {
        inner!(self, power_off_pull_time)
    }

    fn boot_wait(&self) -> Duration {
        inner!(self, boot_wait)
    }

    fn power_down_wait(&self) -> Duration {
        inner!(self, power_down_wait)
    }

    fn reboot_command_wait(&self) -> Duration {
        inner!(self, reboot_command_wait)
    }

    fn command_delay_default(&self) -> Duration {
        inner!(self, command_delay_default)
    }

    fn radio_off_cfun(&self) -> Functionality {
        inner!(self, radio_off_cfun)
    }

    fn reset_hold(&self) -> Duration {
        inner!(self, reset_hold)
    }

    fn max_num_simultaneous_rats(&self) -> u8 {
        inner!(self, max_num_simultaneous_rats)
    }

    fn at_c_fun_reboot_command(&self) -> Functionality {
        inner!(self, at_c_fun_reboot_command)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Generic;

impl ModuleParams for Generic {}
