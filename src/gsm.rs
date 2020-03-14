use atat::prelude::*;
use embedded_hal::digital::v2::OutputPin;
use heapless::{consts, String};
use no_std_net::{IpAddr, Ipv4Addr};

use crate::{
    client::State,
    command::{
        device_lock::{self, types::*},
        general::{self, types::*},
        network_service::{self, types::*},
    },
    error::Error,
    GSMClient,
};

pub trait GSM {
    fn begin(&self, pin: &str) -> Result<(), Error>;
    fn shutdown(&self, secure: bool) -> Result<(), Error>;
    fn get_time(&self) -> Result<(), Error>;
}

impl<C, RST, DTR> GSM for GSMClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn begin(&self, pin: &str) -> Result<(), Error> {
        let pin_status = self.send_at(&device_lock::GetPinStatus)?;

        if pin_status.code != PinStatusCode::Ready {
            log::info!("Pin NOT Ready!\r");
            return Err(Error::Pin);
        }

        while self.send_at(&general::GetCCID).is_err() {}

        while !self
            .send_at(&network_service::GetNetworkRegistrationStatus)?
            .stat
            .registration_ok()?
            .is_access_alive()
        {}

        Ok(())
    }

    fn shutdown(&self, secure: bool) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_time(&self) -> Result<(), Error> {
        unimplemented!()
    }
}
