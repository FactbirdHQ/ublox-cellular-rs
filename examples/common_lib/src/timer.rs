use ublox_cellular::prelude::*;

pub struct SysTimer<const TIMER_HZ: u32> {
    start: std::time::Instant,
    duration: fugit::TimerDurationU32<TIMER_HZ>,
}

impl<const TIMER_HZ: u32> SysTimer<TIMER_HZ> {
    pub fn new() -> Self {
        Self {
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
        Ok(())
    }

    fn wait(&mut self) -> Result<(), Self::Error> {
        loop {
            if std::time::Instant::now() - self.start
                > std::time::Duration::from_millis(self.duration.ticks() as u64)
            {
                break;
            }
        }
        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     extern crate nb;
// }
