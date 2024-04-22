use super::ModuleParams;
use crate::command::mobile_control::types::Functionality;
use embassy_time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct TobyR2;

impl ModuleParams for TobyR2 {
    fn power_on_pull_time(&self) -> Option<Duration> {
        Some(Duration::from_micros(50))
    }
    fn power_off_pull_time(&self) -> Duration {
        Duration::from_millis(1000)
    }
    fn command_delay_default(&self) -> Duration {
        Duration::from_millis(20)
    }
    fn reset_hold(&self) -> Duration {
        Duration::from_millis(50)
    }
    fn max_num_simultaneous_rats(&self) -> u8 {
        // TODO: Is this correct?
        3
    }
}
