use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct SaraR5;

impl ModuleParams for SaraR5 {
    fn power_on_pull_time(&self) -> Option<Duration> {
        Some(Duration::from_millis(1500))
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(2000)
    }
    fn boot_wait(&self) -> Duration {
        Duration::from_secs(6)
    }
    fn power_down_wait(&self) -> Duration {
        Duration::from_secs(20)
    }
    fn reboot_command_wait(&self) -> Duration {
        Duration::from_secs(15)
    }
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(20)
    }
    fn reset_hold(&self) -> Duration {
        Duration::from_millis(150)
    }
    fn at_c_fun_reboot_command(&self) -> Functionality {
        Functionality::SilentResetWithSimReset
    }
}
