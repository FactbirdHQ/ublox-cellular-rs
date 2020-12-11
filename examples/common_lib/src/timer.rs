use core::convert::Infallible;
use embedded_hal::{
    blocking::delay::DelayMs,
    timer::{CountDown, Periodic},
};
use std::time::{Duration, Instant};

#[allow(clippy::module_name_repetitions)]
pub struct SysTimer {
    start: Instant,
    count: u32,
}

impl SysTimer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            count: 0,
        }
    }
}

impl Default for SysTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayMs<u32> for SysTimer {
    type Error = Infallible;

    fn try_delay_ms(&mut self, ms: u32) -> Result<(), Self::Error> {
        self.try_start(ms)?;
        nb::block!(self.try_wait())
    }
}

impl CountDown for SysTimer {
    type Error = Infallible;
    type Time = u32;

    fn try_start<T>(&mut self, count: T) -> Result<(), Self::Error>
    where
        T: Into<Self::Time>,
    {
        self.start = Instant::now();
        self.count = count.into();
        Ok(())
    }

    fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
        if Instant::now() - self.start > Duration::from_millis(u64::from(self.count)) {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl Periodic for SysTimer {}
