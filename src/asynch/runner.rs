use core::str::FromStr;

use crate::{command::Urc, config::CellularConfig};

use super::state::{self, LinkState};
use atat::{asynch::AtatClient, UrcSubscription};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::OutputPin;
use no_std_net::{Ipv4Addr, Ipv6Addr};
use crate::error::Error;
use crate::error::GenericError::Timeout;
use crate::module_timing::{boot_time, reset_time};

use super::AtHandle;

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<
    'd,
    AT: AtatClient,
    C: CellularConfig,
    const URC_CAPACITY: usize,
> {
    ch: state::Runner<'d>,
    at: AtHandle<'d, AT>,
    config: C,
    urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
}

impl<
        'd,
        AT: AtatClient,
        C: CellularConfig,
        const URC_CAPACITY: usize,
    > Runner<'d, AT, C, URC_CAPACITY>
{
    pub(crate) fn new(
        ch: state::Runner<'d>,
        at: AtHandle<'d, AT>,
        config: C,
        urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
    ) -> Self {
        Self {
            ch,
            at,
            config,
            urc_subscription,
        }
    }

    pub(crate) async fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)
        debug!("Initializing module");
        // Hard reset module
        self.reset().await?;


        Ok(())
    }

    pub async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Short Range");
        if let Some(pin) = self.config.reset_pin() {
            pin.set_low().ok();
            Timer::after(reset_time()).await;
            pin.set_high().ok();
            Timer::after(boot_time()).await;
        } else {
            warn!("No reset pin configured");
        }
        Ok(())
    }

    pub async fn restart(&mut self, store: bool) -> Result<(), Error> {

        Ok(())
    }


    pub async fn run(mut self) -> ! {
        loop {
            let event = self.urc_subscription.next_message_pure().await;
            match event {
                // Handle network URCs
                Urc::NetworkDetach => todo!(),
                Urc::MobileStationDetach => todo!(),
                Urc::NetworkDeactivate => todo!(),
                Urc::MobileStationDeactivate => todo!(),
                Urc::NetworkPDNDeactivate => todo!(),
                Urc::MobileStationPDNDeactivate => todo!(),
                Urc::SocketDataAvailable(_) => todo!(),
                Urc::SocketDataAvailableUDP(_) => todo!(),
                Urc::DataConnectionActivated(_) => todo!(),
                Urc::DataConnectionDeactivated(_) => todo!(),
                Urc::SocketClosed(_) => todo!(),
                Urc::MessageWaitingIndication(_) => todo!(),
                Urc::ExtendedPSNetworkRegistration(_) => todo!(),
                Urc::HttpResponse(_) => todo!(),
            };
        }
    }
}
