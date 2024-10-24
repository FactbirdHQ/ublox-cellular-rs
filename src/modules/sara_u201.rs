use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SaraU201;

impl ModuleParams for SaraU201 {
    fn power_on_pull_time(&self) -> Duration {
        Duration::from_millis(1)
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(1500)
    }
    fn power_down_wait(&self) -> Duration {
        Duration::from_secs(5)
    }
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(20)
    }
    fn reset_hold(&self) -> Duration {
        Duration::from_millis(75)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        2
    }
}
