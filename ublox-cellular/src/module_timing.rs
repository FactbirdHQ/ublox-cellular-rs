#![allow(clippy::if_same_then_else)]

use fugit::ExtU32;
use fugit::TimerDurationU32;

/// Low time of `PWR_ON` pin to trigger module switch on from power off mode
pub fn pwr_on_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        150.millis()
    } else if cfg!(feature = "toby-r2") {
        50.micros()
    } else {
        50.micros()
    }
}

/// Low time of `PWR_ON` pin to trigger module graceful switch off
pub fn pwr_off_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        1500.millis()
    } else if cfg!(feature = "toby-r2") {
        1.secs()
    } else {
        1.secs()
    }
}

/// Low time of `RESET_N` pin to trigger module reset (reboot)
pub fn reset_time<const TIMER_HZ: u32>() -> TimerDurationU32<TIMER_HZ> {
    if cfg!(feature = "lara-r6") {
        10.millis()
    } else if cfg!(feature = "toby-r2") {
        50.millis()
    } else {
        50.millis()
    }
}

/// Low time of `RESET_N` pin to trigger module abrupt emergency switch off
///
/// NOTE: Not all modules support this operation from `RESET_N`
pub fn kill_time<const TIMER_HZ: u32>() -> Option<TimerDurationU32<TIMER_HZ>> {
    if cfg!(feature = "lara-r6") {
        Some(10.secs())
    } else {
        None
    }
}
