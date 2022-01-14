use ublox_cellular::fugit;
use ublox_cellular::prelude::*;

pub struct SysTimer<const TIMER_HZ: u32> {
    description: String,
    start: std::time::Instant,
    duration: fugit::TimerDurationU32<TIMER_HZ>,
}

impl<const TIMER_HZ: u32> SysTimer<TIMER_HZ> {
    pub fn new(description: &str) -> Self {
        Self {
            description: description.into(),
            start: std::time::Instant::now(),
            duration: fugit::TimerDurationU32::millis(0),
        }
    }
}

impl<const TIMER_HZ: u32> Clock<TIMER_HZ> for SysTimer<TIMER_HZ> {
    type Error = std::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        let millis = self.start.elapsed().as_millis();
        fugit::TimerInstantU32::from_ticks(millis as u32)
    }

    fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        self.start = std::time::Instant::now();
        self.duration = duration.convert();
        log::debug!(
            "[{}] start {:?} duration {:?}",
            self.description,
            self.start,
            self.duration
        );
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        if std::time::Instant::now() - self.start
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
