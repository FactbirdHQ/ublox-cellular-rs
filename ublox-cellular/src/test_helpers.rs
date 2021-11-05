//! This module is required in order to satisfy the requirements of defmt, while running tests.
//! Note that this will cause all log `defmt::` log statements to be thrown away.
use atat::{AtatClient, Clock};
use core::ptr::NonNull;
use fugit::ExtU32;

#[defmt::global_logger]
struct Logger;
impl defmt::Write for Logger {
    fn write(&mut self, _bytes: &[u8]) {}
}

unsafe impl defmt::Logger for Logger {
    fn acquire() -> Option<NonNull<dyn defmt::Write>> {
        Some(NonNull::from(&Logger as &dyn defmt::Write))
    }

    unsafe fn release(_: NonNull<dyn defmt::Write>) {}
}

defmt::timestamp!("");

#[export_name = "_defmt_panic"]
fn panic() -> ! {
    panic!()
}

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
    ) -> nb::Result<A::Response, atat::Error<A::Error>> {
        todo!()
    }

    fn peek_urc_with<URC: atat::AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, f: F) {
        if let Some(urc) = URC::parse(b"+UREG:0") {
            if f(urc) {
                self.n_urcs_dequeued += 1;
            }
        }
    }

    fn check_response<A: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        _cmd: &A,
    ) -> nb::Result<A::Response, atat::Error<A::Error>> {
        todo!()
    }

    fn get_mode(&self) -> atat::Mode {
        todo!()
    }

    fn reset(&mut self) {}
}

#[derive(Debug)]
pub struct MockTimer<const TIMER_HZ: u32> {
    forced_ms_time: Option<fugit::TimerInstantU32<TIMER_HZ>>,
    start: std::time::Instant,
    duration: fugit::TimerDurationU32<TIMER_HZ>,
}

impl<const TIMER_HZ: u32> MockTimer<TIMER_HZ> {
    pub fn new(forced_ms_time: Option<fugit::TimerInstantU32<TIMER_HZ>>) -> Self {
        Self {
            forced_ms_time,
            start: std::time::Instant::now(),
            duration: fugit::TimerDurationU32::millis(0),
        }
    }
}

impl<const TIMER_HZ: u32> Clock<TIMER_HZ> for MockTimer<TIMER_HZ> {
    type Error = std::convert::Infallible;

    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
        match self.forced_ms_time {
            Some(ts) => ts,
            None => {
                let millis = self.start.elapsed().as_millis();
                fugit::TimerInstantU32::from_ticks(millis as u32)
            }
        }
    }

    fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
        self.start = std::time::Instant::now();
        self.duration = duration.convert();
        Ok(())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        if std::time::Instant::now() - self.start
            > std::time::Duration::from_millis(self.duration.ticks() as u64)
        {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
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
