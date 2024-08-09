use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SaraR412m;

impl ModuleParams for SaraR412m {
    fn power_on_pull_time(&self) -> Option<Duration> {
        Some(Duration::from_millis(300))
    }

    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(2000)
    }
    fn boot_wait(&self) -> Duration {
        Duration::from_secs(6)
    }
    fn reboot_command_wait(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        3
    }
    fn at_c_fun_reboot_command(&self) -> Functionality {
        Functionality::SilentReset
    }
}
