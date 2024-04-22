use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct SaraR410m;

impl ModuleParams for SaraR410m {
    fn power_on_pull_time(&self) -> Option<Duration> {
        Some(Duration::from_millis(300))
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(2000)
    }
    fn boot_wait(&self) -> Duration {
        Duration::from_secs(6)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        2
    }
}
