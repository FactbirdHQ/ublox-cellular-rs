use atat::blocking::AtatClient;
use fugit::ExtU32;
use fugit_timer::Timer;

#[derive(Debug)]
pub struct MockAtClient {
    pub n_urcs_dequeued: u8,
}

impl MockAtClient {
    pub fn new(n_urcs_dequeued: u8) -> Self {
        Self { n_urcs_dequeued }
    }
}

impl AtatClient for MockAtClient {
    fn send<A: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        _cmd: &A,
    ) -> Result<A::Response, atat::Error> {
        todo!()
    }

    fn try_read_urc_with<Urc: atat::AtatUrc, F: for<'b> FnOnce(Urc::Response, &'b [u8]) -> bool>(
        &mut self,
        handle: F,
    ) -> bool {
        if let Some(urc) = Urc::parse(b"+UREG:0") {
            if handle(urc, b"") {
                self.n_urcs_dequeued += 1;
            }
        }
        true
    }

    fn max_urc_len() -> usize {
        todo!()
    }
}

#[derive(Debug)]
pub struct MockTimer<const TIMER_HZ: u32> {
    forced_ms_time: Option<fugit::TimerInstantU32<TIMER_HZ>>,
    monotonic: std::time::Instant,
    start: Option<std::time::Instant>,
    duration: fugit::TimerDurationU32<TIMER_HZ>,
}

impl<const TIMER_HZ: u32> MockTimer<TIMER_HZ> {
    pub fn new(forced_ms_time: Option<fugit::TimerInstantU32<TIMER_HZ>>) -> Self {
        Self {
            forced_ms_time,
            monotonic: std::time::Instant::now(),
            start: None,
            duration: fugit::TimerDurationU32::millis(0),
        }
    }
}

impl<const TIMER_HZ: u32> Timer<TIMER_HZ> for MockTimer<TIMER_HZ> {
    type Error = std::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        match self.forced_ms_time {
            Some(ts) => ts,
            None => {
                let millis = self.monotonic.elapsed().as_millis();
                fugit::TimerInstantU32::from_ticks(millis as u32)
            }
        }
    }

    fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        self.start = Some(std::time::Instant::now());
        self.duration = duration.convert();
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
            if start.elapsed() > std::time::Duration::from_millis(u64::from(self.duration.ticks()))
            {
                Ok(())
            } else {
                Err(nb::Error::WouldBlock)
            }
        } else {
            Ok(())
        }
    }
}

mod tests {
    use super::*;

    const TIMER_HZ: u32 = 1000;

    #[test]
    fn mock_timer_works() {
        let now = std::time::Instant::now();

        let mut timer: MockTimer<TIMER_HZ> = MockTimer::new(None);
        timer.start(1.secs()).unwrap();
        nb::block!(timer.wait()).unwrap();

        assert!(now.elapsed().as_millis() >= 1_000);
    }
}
