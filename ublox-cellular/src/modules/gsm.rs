use crate::{
    command::{
        device_lock::{self, types::*},
        general,
        ip_transport_layer::{self, types::*},
        mobile_control::{self, responses::*, types::*},
    },
    error::Error,
    GsmClient,
};
use embedded_hal::{blocking::delay::DelayMs, digital::{OutputPin, InputPin}};
use heapless::{ArrayLength, Bucket, Pos, PowerOfTwo};

pub trait GSM {
    fn begin(&self) -> Result<(), Error>;
    fn shutdown(&self, secure: bool) -> Result<(), Error>;
    fn get_time(&self) -> Result<DateTime, Error>;
}

impl<C, DLY, N, L, RST, DTR, PWR, VINT> GSM for GsmClient<C, DLY, N, L, RST, DTR, PWR, VINT>
where
    C: atat::AtatClient,
    DLY: DelayMs<u32>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>
        + PowerOfTwo,
    L: ArrayLength<u8>,
{
    fn begin(&self) -> Result<(), Error> {

        // let pin_status = self.send_at(&device_lock::GetPinStatus)?;

        // match pin_status.code {
        //     PinStatusCode::SimPin => {
        //         self.send_at(&device_lock::SetPin {
        //             pin: self.config.pin(),
        //         })?;
        //     }
        //     PinStatusCode::PhSimPin
        //     | PinStatusCode::SimPuk
        //     | PinStatusCode::SimPin2
        //     | PinStatusCode::SimPuk2
        //     | PinStatusCode::PhNetPin
        //     | PinStatusCode::PhNetSubPin
        //     | PinStatusCode::PhSpPin
        //     | PinStatusCode::PhCorpPin => {
        //         defmt::info!("Pin NOT Ready!");
        //         return Err(Error::Pin);
        //     }
        //     PinStatusCode::Ready => {}
        // }

        // while self.send_at(&general::GetCCID).is_err() {}

        // if self.config.try_borrow()?.hex_mode {
        //     self.send_at(&ip_transport_layer::SetHexMode {
        //         hex_mode_disable: HexMode::Enabled,
        //     })?;
        // } else {
        //     self.send_at(&ip_transport_layer::SetHexMode {
        //         hex_mode_disable: HexMode::Disabled,
        //     })?;
        // }

        Ok(())
    }

    fn shutdown(&self, secure: bool) -> Result<(), Error> {
        // if secure {
        //     self.send_at(&mobile_control::ModuleSwitchOff)?;
        // }
        Ok(())
    }

    fn get_time(&self) -> Result<DateTime, Error> {
        self.send_at(&mobile_control::GetClock)
    }
}
