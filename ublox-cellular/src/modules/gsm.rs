use crate::{
    command::{
        device_lock::{self, types::*},
        general,
        ip_transport_layer::{self, types::*},
        mobile_control::{self, responses::*, types::*},
        network_service,
    },
    error::Error,
    GsmClient, State,
};
use embedded_hal::digital::v2::OutputPin;
use heapless::ArrayLength;

pub trait GSM {
    fn begin(&self) -> Result<(), Error>;
    fn shutdown(&self, secure: bool) -> Result<(), Error>;
    fn get_time(&self) -> Result<DateTime, Error>;
}

impl<C, RST, DTR, N, L> GSM for GsmClient<C, RST, DTR, N, L>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    fn begin(&self) -> Result<(), Error> {
        self.state.set(State::Registering);

        let pin_status = self.send_at(&device_lock::GetPinStatus)?;

        match pin_status.code {
            PinStatusCode::SimPin => {
                self.send_at(&device_lock::SetPin {
                    pin: self.config.pin(),
                })?;
            }
            PinStatusCode::PhSimPin
            | PinStatusCode::SimPuk
            | PinStatusCode::SimPin2
            | PinStatusCode::SimPuk2
            | PinStatusCode::PhNetPin
            | PinStatusCode::PhNetSubPin
            | PinStatusCode::PhSpPin
            | PinStatusCode::PhCorpPin => {
                defmt::info!("Pin NOT Ready!");
                return Err(Error::Pin);
            }
            PinStatusCode::Ready => {}
        }

        while self.send_at(&general::GetCCID).is_err() {}

        self.send_at(&ip_transport_layer::SetHexMode {
            hex_mode_disable: HexMode::Enabled,
        })?;

        self.send_at(&mobile_control::SetAutomaticTimezoneUpdate {
            on_off: AutomaticTimezone::EnabledLocal,
        })?;

        while !self
            .send_at(&network_service::GetNetworkRegistrationStatus)?
            .stat
            .registration_ok()?
            .is_access_alive()
        {}

        self.state.set(State::Registered);

        Ok(())
    }

    fn shutdown(&self, secure: bool) -> Result<(), Error> {
        if secure {
            self.send_at(&mobile_control::ModuleSwitchOff)?;
        }
        Ok(())
    }

    fn get_time(&self) -> Result<DateTime, Error> {
        self.send_at(&mobile_control::GetClock)
    }
}
