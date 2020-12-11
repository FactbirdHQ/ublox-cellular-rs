use embedded_hal::{timer::{CountDown, Periodic}, blocking::delay::DelayMs};
use std::time::{Duration, Instant};
use core::convert::Infallible;

pub struct SysTimer {
    start: Instant,
    count: u32,
}

impl SysTimer {
    pub fn new() -> SysTimer {
        SysTimer {
            start: Instant::now(),
            count: 0,
        }
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
        if Instant::now() - self.start > Duration::from_millis(self.count as u64) {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl Periodic for SysTimer {}
