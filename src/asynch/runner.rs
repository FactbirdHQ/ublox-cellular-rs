use core::str::FromStr;

use crate::{command::Urc, config::CellularConfig};

use super::state::{self, LinkState};
use atat::{asynch::AtatClient, UrcSubscription};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::OutputPin;
use no_std_net::{Ipv4Addr, Ipv6Addr};

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

    async fn wait_startup(&mut self, timeout: Duration) -> Result<(), Error> {
        let fut = async {
            loop {
                match self.urc_subscription.next_message_pure().await {
                    Urc::StartUp => return,
                    _ => {}
                }
            }
        };

        with_timeout(timeout, fut).await.map_err(|_| Error::Timeout)
    }

    pub async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Short Range");
        self.reset.set_low().ok();
        Timer::after(Duration::from_millis(100)).await;
        self.reset.set_high().ok();

        self.wait_startup(Duration::from_secs(4)).await?;

        Ok(())
    }

    pub async fn restart(&mut self, store: bool) -> Result<(), Error> {
        warn!("Soft resetting Ublox Short Range");
        if store {
            self.at.send(StoreCurrentConfig).await?;
        }

        self.at.send(RebootDCE).await?;

        Timer::after(Duration::from_millis(3500)).await;

        Ok(())
    }

    pub async fn is_link_up(&mut self) -> Result<bool, Error> {
        // Determine link state
        // let link_state = match self.wifi_connection {
        //     Some(ref conn)
        //         if conn.network_up && matches!(conn.wifi_state, WiFiState::Connected) =>
        //     {
        //         LinkState::Up
        //     }
        //     _ => LinkState::Down,
        // };

        self.ch.set_link_state(link_state);

        Ok(link_state == LinkState::Up)
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
