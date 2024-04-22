use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct SaraR422;

impl ModuleParams for SaraR422 {
    fn power_on_pull_time(&self) -> Duration {
        Duration::from_millis(300)
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(2000)
    }
    fn reboot_command_wait(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(20)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        3
    }
}
