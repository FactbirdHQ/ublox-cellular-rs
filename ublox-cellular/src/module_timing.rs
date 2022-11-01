use fugit::ExtU32;
use fugit::TimerDurationU32;

/// Low time of `PWR_ON` pin to trigger module switch on from power off mode
pub fn pwr_on_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        150.millis()
    } else {
        50.micros()
    }
}

/// Low time of `PWR_ON` pin to trigger module graceful switch off
pub fn pwr_off_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        1500.millis()
    } else {
        1.secs()
    }
}

/// Low time of `RESET_N` pin to trigger module reset (reboot)
pub fn reset_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        10.millis()
    } else {
        50.millis()
    }
}

/// Low time of `RESET_N` pin to trigger module abrupt emergency switch off
pub fn kill_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        10.secs()
    } else {
        10.secs()
    }
}
