#![allow(clippy::if_same_then_else)]

use embassy_time::Duration;

/// Low time of `PWR_ON` pin to trigger module switch on from power off mode
pub fn pwr_on_time() -> Duration {
    if cfg!(feature = "lara-r6") {
        Duration::from_millis(150)
    } else if cfg!(feature = "sara-r5") {
        Duration::from_millis(150)
    } else if cfg!(feature = "toby-r2") {
        Duration::from_micros(50)
    } else {
        Duration::from_micros(50)
    }
}

/// Low time of `PWR_ON` pin to trigger module graceful switch off
pub fn pwr_off_time() -> Duration {
    if cfg!(feature = "lara-r6") {
        Duration::from_millis(1500)
    } else if cfg!(feature = "sara-r5") {
        Duration::from_millis(5000)
    } else if cfg!(feature = "toby-r2") {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(1)
    }
}

/// Low time of `RESET_N` pin to trigger module reset (reboot)
pub fn reset_time() -> Duration {
    if cfg!(feature = "lara-r6") {
        Duration::from_millis(10)
    } else if cfg!(feature = "toby-r2") {
        Duration::from_millis(50)
    } else if cfg!(feature = "sara-r5") {
        Duration::from_millis(100)
    } else {
        Duration::from_millis(50)
    }
}

/// Time to wait for module to boot
pub fn boot_time() -> Duration {
    if cfg!(feature = "sara-r5") {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(1)
    }
}

/// Low time of `RESET_N` pin to trigger module abrupt emergency switch off
///
/// NOTE: Not all modules support this operation from `RESET_N`
pub fn kill_time() -> Option<Duration> {
    if cfg!(feature = "lara-r6") {
        Some(Duration::from_secs(10))
    } else {
        None
    }
}
