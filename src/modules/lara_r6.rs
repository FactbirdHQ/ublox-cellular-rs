use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct LaraR6;

impl ModuleParams for LaraR6 {
    fn power_on_pull_time(&self) -> Option<Duration> {
        Some(Duration::from_millis(300))
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(2000)
    }
    fn boot_wait(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn reboot_command_wait(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(20)
    }
    fn reset_hold(&self) -> Duration {
        Duration::from_millis(150)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        3
    }
}
