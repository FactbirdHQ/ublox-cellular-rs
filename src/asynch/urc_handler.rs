use atat::{UrcChannel, UrcSubscription};

use crate::command::Urc;

use super::{runner::URC_SUBSCRIBERS, state};

pub struct UrcHandler<'a, 'b, const URC_CAPACITY: usize> {
    ch: &'b state::Runner<'a>,
    urc_subscription: UrcSubscription<'a, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<'a, 'b, const URC_CAPACITY: usize> UrcHandler<'a, 'b, URC_CAPACITY> {
    pub fn new(
        ch: &'b state::Runner<'a>,
        urc_channel: &'a UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    ) -> Self {
        Self {
            ch,
            urc_subscription: urc_channel.subscribe().unwrap(),
        }
    }

    pub async fn run(&mut self) -> ! {
        loop {
            let event = self.urc_subscription.next_message_pure().await;
            self.handle_urc(event).await;
        }
    }

    async fn handle_urc(&mut self, event: Urc) {
        match event {
            // Handle network URCs
            Urc::NetworkDetach => warn!("Network detached"),
            Urc::MobileStationDetach => warn!("Mobile station detached"),
            Urc::NetworkDeactivate => warn!("Network deactivated"),
            Urc::MobileStationDeactivate => warn!("Mobile station deactivated"),
            Urc::NetworkPDNDeactivate => warn!("Network PDN deactivated"),
            Urc::MobileStationPDNDeactivate => warn!("Mobile station PDN deactivated"),
            #[cfg(feature = "internal-network-stack")]
            Urc::SocketDataAvailable(_) => warn!("Socket data available"),
            #[cfg(feature = "internal-network-stack")]
            Urc::SocketDataAvailableUDP(_) => warn!("Socket data available UDP"),
            Urc::DataConnectionActivated(_) => warn!("Data connection activated"),
            Urc::DataConnectionDeactivated(_) => {
                warn!("Data connection deactivated");
                #[cfg(not(feature = "use-upsd-context-activation"))]
                if self.ch.get_profile_state() == crate::registration::ProfileState::ShouldBeUp {
                    // Set the state so that, should we re-register with the
                    // network, we will reactivate the internal profile
                    self.ch
                        .set_profile_state(crate::registration::ProfileState::RequiresReactivation);
                }
            }
            #[cfg(feature = "internal-network-stack")]
            Urc::SocketClosed(_) => warn!("Socket closed"),
            Urc::MessageWaitingIndication(_) => warn!("Message waiting indication"),
            Urc::ExtendedPSNetworkRegistration(_) => warn!("Extended PS network registration"),
            Urc::HttpResponse(_) => warn!("HTTP response"),
            Urc::NetworkRegistration(reg) => {
                self.ch
                    .update_registration_with(|state| state.compare_and_set(reg.into()));
            }
            Urc::GPRSNetworkRegistration(reg) => {
                self.ch
                    .update_registration_with(|state| state.compare_and_set(reg.into()));
            }
            Urc::EPSNetworkRegistration(reg) => {
                self.ch
                    .update_registration_with(|state| state.compare_and_set(reg.into()));
            }
        };
    }
}
