use core::convert::TryInto;

use crate::services::data::socket::Socket;
use crate::{client::Device, error::Error as DeviceError};
use atat::blocking::AtatClient;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_time::{Clock, duration::{Generic, Milliseconds}};
use heapless::{ArrayLength, Bucket, Pos};

impl<C, CLK, N, L, RST, DTR, PWR, VINT> Device<C, CLK, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    CLK: fugit_timer::Timer,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<Socket<L, CLK>>> + ArrayLength<Bucket<u8, usize>> + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn location_service(&mut self) -> nb::Result<LocationService, DeviceError> {
        self.spin()?;

        Ok(LocationService)
    }
}

/// Empty location service, to showcase how multiple services can be implemented!
pub struct LocationService;
