use super::ModuleParams;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    // After the power-off pulse the module drops VInt within ~1-2s when it
    // actually powers off. The default (35s) is the time `power_down` spins
    // waiting for VInt to fall; on boards where VInt does not reflect power-off
    // that wait is burned in full on every power-down. 5s is ample to observe a
    // real power-off while capping that wait.
    fn power_down_wait(&self) -> Duration {
        Duration::from_secs(5)
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
