//! This module is required in order to satisfy the requirements of defmt, while running tests.
//! Note that this will cause all log `defmt::` log statements to be thrown away.
use super::Clock;
use atat::AtatClient;
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
pub struct MockTimer<const FREQ_HZ: u32> {
    forced_ms_time: Option<fugit::TimerInstantU32<FREQ_HZ>>,
    start: std::time::Instant,
    millis: fugit::MillisDurationU32,
}

impl<const FREQ_HZ: u32> MockTimer<FREQ_HZ> {
    pub fn new(forced_ms_time: Option<fugit::TimerInstantU32<FREQ_HZ>>) -> Self {
        Self {
            forced_ms_time,
            start: std::time::Instant::now(),
            millis: fugit::MillisDurationU32::millis(0),
        }
    }
}

impl<const FREQ_HZ: u32> Clock<FREQ_HZ> for MockTimer<FREQ_HZ> {
    fn now(&mut self) -> fugit::TimerInstantU32<FREQ_HZ> {
        match self.forced_ms_time {
            Some(ts) => ts,
            None => {
                let millis = self.start.elapsed().as_millis();
                fugit::TimerInstantU32::from_ticks(millis as u32)
            }
        }
    }

    fn start<T>(&mut self, count: T) -> Result<(), super::ClockError>
    where
        T: Into<fugit::MillisDurationU32>,
    {
        self.start = std::time::Instant::now();
        self.millis = count.into();
        Ok(())
    }

    fn wait(&mut self) -> Result<(), super::ClockError> {
        loop {
            if std::time::Instant::now() - self.start
                > std::time::Duration::from_millis(self.millis.ticks() as u64)
            {
                break;
            }
        }
        Ok(())
    }
}

mod tests {
    use super::*;

    const FREQ_HZ: u32 = 1000;

    #[test]
    fn mock_timer_works() {
        let now = std::time::Instant::now();

        let mut timer: MockTimer<FREQ_HZ> = MockTimer::new(None);
        timer.start(1.secs()).unwrap();
        timer.wait().unwrap();

        assert!(now.elapsed().as_millis() >= 1_000);
    }
}
