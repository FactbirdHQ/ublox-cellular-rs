use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LenaR8;

impl ModuleParams for LenaR8 {
    fn power_on_pull_time(&self) -> Duration {
        Duration::from_millis(2000)
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(3100)
    }
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(20)
    }
    fn reset_hold(&self) -> Duration {
        Duration::from_millis(50)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        2
    }
    fn at_c_fun_reboot_command(&self) -> Functionality {
        Functionality::SilentResetWithSimReset
    }
}
