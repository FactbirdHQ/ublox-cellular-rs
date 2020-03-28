use embedded_hal::digital::v2::OutputPin;

use crate::{
    command::{
        device_lock::{self, types::*},
        general,
        ip_transport_layer::{self, types::*},
        mobile_control::{self, responses::*, types::*},
        network_service,
    },
    error::Error,
    GSMClient, GSMState,
};

pub trait GSM {
    fn begin(&self, pin: &str) -> Result<(), Error>;
    fn shutdown(&self, secure: bool) -> Result<(), Error>;
    fn get_time(&self) -> Result<DateTimeString, Error>;
}

impl<C, RST, DTR> GSM for GSMClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn begin(&self, pin: &str) -> Result<(), Error> {
        self.set_state(GSMState::Registering)?;

        let pin_status = self.send_at(&device_lock::GetPinStatus)?;

        match pin_status.code {
            PinStatusCode::SimPin => {
                self.send_at(&device_lock::SetPin { pin })?;
            }
            PinStatusCode::PhSimPin
            | PinStatusCode::SimPuk
            | PinStatusCode::SimPin2
            | PinStatusCode::SimPuk2
            | PinStatusCode::PhNetPin
            | PinStatusCode::PhNetSubPin
            | PinStatusCode::PhSpPin
            | PinStatusCode::PhCorpPin => {
                log::info!("Pin NOT Ready!");
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

        self.set_state(GSMState::Registered)?;

        Ok(())
    }

    fn shutdown(&self, secure: bool) -> Result<(), Error> {
        if secure {
            self.send_at(&mobile_control::ModuleSwitchOff)?;
        }
        Ok(())
    }

    fn get_time(&self) -> Result<DateTimeString, Error> {
        self.send_at(&mobile_control::GetClock)


        // let parts: Vec<u8, consts::U7> = datetime
        //     .time
        //     .split(|c| c == '/' || c == ',' || c == ':' || c == '+'  || c == '-')
        //     .map(|p| p.parse::<u8>().unwrap())
        //     .collect();

        // let timezone: i8 = if datetime.time.contains("+") {
        //     parts[6] as i8
        // } else {
        //     -(parts[6] as i8)
        // };

        // Ok(DateTime {
        //     year: parts[0],
        //     month: parts[1],
        //     day: parts[2],
        //     hours: parts[3],
        //     minutes: parts[4],
        //     seconds: parts[5],
        //     timezone
        // })
    }
}
