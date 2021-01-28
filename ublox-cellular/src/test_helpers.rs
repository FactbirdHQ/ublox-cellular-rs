use atat::AtatClient;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::timer::CountDown;

pub struct MockTimer {
    pub time: Option<u32>,
}

impl MockTimer {
    pub fn new() -> Self {
        MockTimer { time: None }
    }
}

impl CountDown for MockTimer {
    type Error = core::convert::Infallible;
    type Time = u32;
    fn try_start<T>(&mut self, count: T) -> Result<(), Self::Error>
    where
        T: Into<Self::Time>,
    {
        self.time = Some(count.into());
        Ok(())
    }
    fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
        self.time = None;
        Ok(())
    }
}

impl DelayMs<u32> for MockTimer {
    type Error = core::convert::Infallible;

    fn try_delay_ms(&mut self, ms: u32) -> Result<(), Self::Error> {
        self.try_start(ms)?;
        nb::block!(self.try_wait())
    }
}

pub struct MockAtClient {}

impl MockAtClient {
    pub fn new() -> Self {
        Self {}
    }
}

impl AtatClient for MockAtClient {
    fn send<A: atat::AtatCmd>(&mut self, _cmd: &A) -> nb::Result<A::Response, atat::Error> {
        todo!()
    }

    fn peek_urc_with<URC: atat::AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, _f: F) {}

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
