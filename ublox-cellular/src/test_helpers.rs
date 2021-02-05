use atat::AtatClient;
use embedded_time::{rate::Fraction, Clock, Instant};

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
    fn send<A: atat::AtatCmd>(&mut self, _cmd: &A) -> nb::Result<A::Response, atat::Error> {
        todo!()
    }

    fn peek_urc_with<URC: atat::AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, f: F) {
        if let Ok(urc) = URC::parse(b"+UREG:0") {
            if f(urc) {
                self.n_urcs_dequeued += 1;
            }
        }
    }

    fn check_response<A: atat::AtatCmd>(
        &mut self,
        _cmd: &A,
    ) -> nb::Result<A::Response, atat::Error> {
        todo!()
    }

    fn get_mode(&self) -> atat::Mode {
        todo!()
    }
}

#[derive(Debug)]
pub struct MockTimer {
    forced_ms_time: Option<u32>,
    start_time: std::time::SystemTime,
}

impl MockTimer {
    pub fn new(forced_ms_time: Option<u32>) -> Self {
        Self {
            forced_ms_time,
            start_time: std::time::SystemTime::now(),
        }
    }
}

impl Clock for MockTimer {
    type T = u32;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1000);

    fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
        Ok(Instant::new(self.forced_ms_time.unwrap_or_else(|| {
            self.start_time.elapsed().unwrap().as_millis() as u32
        })))
    }
}

mod tests {
    use super::*;
    use embedded_time::duration::*;

    #[test]
    fn mock_timer_works() {
        let now = std::time::SystemTime::now();

        let timer = MockTimer::new(None);
        timer
            .new_timer(1_u32.seconds())
            .start()
            .unwrap()
            .wait()
            .unwrap();

        assert!(now.elapsed().unwrap().as_millis() >= 1_000);
    }
}
