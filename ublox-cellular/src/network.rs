use crate::{services::data::ContextState, command::network_service::GetOperatorSelection, command::network_service::responses::OperatorSelection, command::{
        ip_transport_layer,
        network_service::GetNetworkRegistrationStatus,
        psn::{
            self, types::PSEventReportingMode, GetEPSNetworkRegistrationStatus,
            GetGPRSNetworkRegistrationStatus, SetPacketSwitchedEventReporting,
        },
        Urc,
    }, error::GenericError, state::{Event, NetworkStatus, ServiceStatus}};
use atat::{atat_derive::AtatLen, AtatClient};
use core::{
    cell::{BorrowError, BorrowMutError, Cell, RefCell},
    ops::DerefMut,
};
use hash32_derive::Hash32;
use serde::{Deserialize, Serialize};

#[derive(Debug, defmt::Format)]
pub enum Error {
    Generic(GenericError),
    AT(atat::Error),
    RegistrationDenied,
    UnknownProfile,
    ActivationFailed,
    _Unknown,
}

impl From<BorrowMutError> for Error {
    fn from(e: BorrowMutError) -> Self {
        Error::Generic(e.into())
    }
}

impl From<BorrowError> for Error {
    fn from(e: BorrowError) -> Self {
        Error::Generic(e.into())
    }
}

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash32, Serialize, Deserialize, AtatLen, defmt::Format,
)]
pub struct ProfileId(pub u8);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, AtatLen, defmt::Format)]
pub struct ContextId(pub u8);

pub struct AtTx<C> {
    urc_attempts: Cell<u8>,
    max_urc_attempts: u8,
    client: RefCell<C>,
}

impl<C: AtatClient> AtTx<C> {
    pub fn new(client: C, max_urc_attempts: u8) -> Self {
        Self {
            urc_attempts: Cell::new(0),
            max_urc_attempts,
            client: RefCell::new(client),
        }
    }

    pub fn handle_urc<F: FnOnce(Urc) -> bool>(&self, f: F) -> Result<(), Error> {
        self.client
            .try_borrow_mut()?
            .peek_urc_with::<Urc, _>(|urc| {
                if !f(urc) {
                    let a = self.urc_attempts.get();
                    if a < self.max_urc_attempts {
                        self.urc_attempts.set(a + 1);
                        return false;
                    } else {
                        defmt::warn!("Dropping stale URC!");
                    }
                }
                self.urc_attempts.set(0);
                true
            });

        Ok(())
    }
}

pub struct Network<C> {
    pub(crate) network_status: RefCell<NetworkStatus>,
    pub(crate) context_state: Cell<ContextState>,
    pub(crate) at_tx: AtTx<C>,
}

impl<C> Network<C>
where
    C: AtatClient,
{
    pub(crate) fn new(at_tx: AtTx<C>) -> Self {
        Network {
            network_status: RefCell::new(NetworkStatus::new()),
            context_state: Cell::new(ContextState::Setup),
            at_tx,
        }
    }

    pub fn get_event(&self) -> Result<Option<Event>, Error> {
        Ok(self.network_status.try_borrow_mut()?.events.dequeue())
    }

    pub fn push_event(&self, event: Event) -> Result<(), Error> {
        Ok(self.network_status.try_borrow_mut()?.push_event(event))
    }

    pub fn clear_events(&self) -> Result<(), Error> {
        let mut status = self.network_status.try_borrow_mut()?;
        while !status.events.is_empty() {
            status.events.dequeue();
        }
        Ok(())
    }

    pub fn is_registered(&self) -> Result<ServiceStatus, Error> {
        let mut status = self.network_status.try_borrow_mut()?;

        status.compare_and_set(
            self.send_internal(&GetNetworkRegistrationStatus, true)?
                .into(),
        );

        status.compare_and_set(
            self.send_internal(&GetGPRSNetworkRegistrationStatus, true)?
                .into(),
        );

        if !status.ps_reg_status.is_registered() {
            status.compare_and_set(
                self.send_internal(&GetEPSNetworkRegistrationStatus, true)?
                    .into(),
            );
        }

        let mut service_status: ServiceStatus = status.deref_mut().into();

        let OperatorSelection { mode, oper, act } =
            self.send_internal(&GetOperatorSelection, true)?;

        service_status.network_registration_mode = mode;
        service_status.operator = oper;

        if let Some(act) = act {
            service_status.rat = act;
        }

        Ok(service_status)
    }

    pub fn set_packet_domain_event_reporting(&self, enable: bool) -> Result<(), Error> {
        let mode = if enable {
            PSEventReportingMode::DiscardUrcs
        } else {
            PSEventReportingMode::CircularBufferUrcs
        };

        self.send_internal(&SetPacketSwitchedEventReporting { mode, bfr: None }, true)?;

        Ok(())
    }

    pub(crate) fn handle_urc(&self) -> Result<(), Error> {
        self.at_tx.handle_urc(|urc| {
            match urc {
                Urc::NetworkDetach => {
                    defmt::warn!("Network Detach URC!");
                }
                Urc::MobileStationDetach => {
                    defmt::warn!("ME Detach URC!");
                }
                Urc::NetworkDeactivate => {
                    defmt::warn!("Network Deactivate URC!");
                }
                Urc::MobileStationDeactivate => {
                    defmt::warn!("ME Deactivate URC!");
                }
                Urc::NetworkPDNDeactivate => {
                    defmt::warn!("Network PDN Deactivate URC!");
                }
                Urc::MobileStationPDNDeactivate => {
                    defmt::warn!("ME PDN Deactivate URC!");
                }
                Urc::ExtendedPSNetworkRegistration(psn::urc::ExtendedPSNetworkRegistration {
                    state,
                }) => {
                    defmt::info!("[URC] ExtendedPSNetworkRegistration {:?}", state);
                }
                Urc::GPRSNetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.network_status.try_borrow_mut() {
                        params.compare_and_set(reg_params.into())
                    }
                }
                Urc::EPSNetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.network_status.try_borrow_mut() {
                        params.compare_and_set(reg_params.into())
                    }
                }
                Urc::NetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.network_status.try_borrow_mut() {
                        params.compare_and_set(reg_params.into())
                    }
                }
                Urc::DataConnectionActivated(psn::urc::DataConnectionActivated {
                    result,
                    ip_addr: _,
                }) => {
                    defmt::info!("[URC] DataConnectionActivated {:u8}", result);
                    if let Ok(mut params) = self.network_status.try_borrow_mut() {
                        params.push_event(Event::DataActive);
                    }
                }
                Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    profile_id,
                }) => {
                    defmt::info!("[URC] DataConnectionDeactivated {:?}", profile_id);
                    if let Ok(mut params) = self.network_status.try_borrow_mut() {
                        params.push_event(Event::DataInactive);
                    }
                }
                Urc::MessageWaitingIndication(_) => {
                    defmt::info!("[URC] MessageWaitingIndication");
                }
                Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                    defmt::info!(
                        "[URC] Socket {:?} closed! Should be followed by one more!",
                        socket
                    );
                    return false;
                }
                _ => return false,
            };
            true
        })
    }

    pub(crate) fn send_internal<A: atat::AtatCmd>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error> {
        if check_urc {
            if let Err(e) = self.handle_urc() {
                defmt::error!("Failed handle URC: {:?}", e);
            }
        }

        self.at_tx
            .client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    match core::str::from_utf8(&req.as_bytes()) {
                        Ok(s) => defmt::error!("{:?}: [{:str}]", ate, s[..s.len() - 2]),
                        Err(_) => defmt::error!(
                            "{:?}: {:?}",
                            ate,
                            core::convert::AsRef::<[u8]>::as_ref(&req.as_bytes())
                        ),
                    };
                    Error::AT(ate)
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AtClient {
        n_urcs_dequeued: u8,
    }

    impl AtatClient for AtClient {
        fn send<A: atat::AtatCmd>(&mut self, _cmd: &A) -> nb::Result<A::Response, atat::Error> {
            unreachable!()
        }

        fn peek_urc_with<URC: atat::AtatUrc, F: FnOnce(URC::Response) -> bool>(&mut self, f: F) {
            if let Ok(urc) = URC::parse(b"+UREG:0") {
                if f(urc) {
                    self.n_urcs_dequeued += 1;
                }
            }
        }

        fn check_response<A: atat::AtatCmd>(
            &mut self,
            _cmd: &A,
        ) -> nb::Result<A::Response, atat::Error> {
            unreachable!()
        }

        fn get_mode(&self) -> atat::Mode {
            unreachable!()
        }
    }

    #[test]
    fn unhandled_urcs() {
        let tx = AtTx::new(AtClient { n_urcs_dequeued: 0 }, 5);

        tx.handle_urc(|_| false).unwrap();
        assert_eq!(tx.client.borrow().n_urcs_dequeued, 0);
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        assert_eq!(tx.client.borrow().n_urcs_dequeued, 1);
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| true).unwrap();
        tx.handle_urc(|_| false).unwrap();
        assert_eq!(tx.client.borrow().n_urcs_dequeued, 2);
    }
}
