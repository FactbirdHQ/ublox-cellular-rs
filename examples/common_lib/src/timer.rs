use ublox_cellular::fugit;
use ublox_cellular::prelude::*;

pub struct SysTimer<const TIMER_HZ: u32> {
    description: String,
    monotonic: std::time::Instant,
    start: Option<std::time::Instant>,
    duration: fugit::TimerDurationU32<TIMER_HZ>,
}

impl<const TIMER_HZ: u32> SysTimer<TIMER_HZ> {
    pub fn new(description: &str) -> Self {
        Self {
            description: description.into(),
            monotonic: std::time::Instant::now(),
            start: None,
            duration: fugit::TimerDurationU32::millis(0),
        }
    }
}

impl<const TIMER_HZ: u32> Clock<TIMER_HZ> for SysTimer<TIMER_HZ> {
    type Error = std::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        let millis = self.monotonic.elapsed().as_millis();
        fugit::TimerInstantU32::from_ticks(millis as u32)
    }

    fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        self.start = Some(std::time::Instant::now());
        self.duration = duration.convert();
        log::debug!(
            "[{}] start {:?} duration {:?}",
            self.description,
            self.start,
            self.duration
        );
        Ok(())
    }

    fn cancel(&mut self) -> Result<(), Self::Error> {
        if self.start.is_some() {
            self.start = None;
        }
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        if let Some(start) = self.start {
            if std::time::Instant::now() - start
                > std::time::Duration::from_millis(self.duration.ticks() as u64)
            {
                log::debug!(
                    "[{}] now {:?} start {:?} duration {:?} {:?}",
                    self.description,
                    std::time::Instant::now(),
                    self.start,
                    self.duration,
                    std::time::Duration::from_millis(self.duration.ticks() as u64)
                );
                Ok(())
            } else {
                Err(nb::Error::WouldBlock)
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fugit::ExtU32;

    #[test]
    fn sys_timer() {
        let now = std::time::Instant::now();

        let mut t: SysTimer<1000> = SysTimer::new("");
        t.start(1.secs()).unwrap();
        nb::block!(t.wait()).unwrap();

        assert!(now.elapsed().as_millis() >= 1000);
    }
}
