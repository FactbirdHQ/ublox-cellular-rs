use crate::services::data::socket::SocketSetItem;
use crate::{client::Device, error::Error as DeviceError};
use atat::AtatClient;
use embedded_hal::{
    blocking::delay::DelayMs,
    digital::{InputPin, OutputPin},
    timer::CountDown,
};
use heapless::{ArrayLength, Bucket, Pos};

impl<C, DLY, N, L, RST, DTR, PWR, VINT> Device<C, DLY, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    DLY: DelayMs<u32> + CountDown,
    DLY::Time: From<u32>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn location_service(&mut self) -> nb::Result<LocationService, DeviceError> {
        self.spin()?;

        Ok(LocationService)
    }
}

/// Empty location service, to showcase how multiple services can be implemented!
pub struct LocationService;
