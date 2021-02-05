use crate::{
    command::{
        mobile_control::{types::Functionality, SetModuleFunctionality},
        network_service::{
            types::OperatorSelectionMode, GetNetworkRegistrationStatus, SetOperatorSelection,
        },
        psn::{
            self, types::PDPContextStatus, GetEPSNetworkRegistrationStatus,
            GetGPRSNetworkRegistrationStatus, GetPDPContextState, SetPDPContextState,
        },
        Urc,
    },
    error::GenericError,
    registration::{self, ConnectionState, RegistrationState},
    services::data::ContextState,
};
use atat::{atat_derive::AtatLen, AtatClient};
use core::{
    cell::{BorrowError, BorrowMutError, Cell, RefCell},
    convert::TryInto,
};
use embedded_time::{duration::*, Clock, TimeError};
use hash32_derive::Hash32;
use heapless::consts;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub enum Error {
    Generic(GenericError),
    AT(atat::Error),
    RegistrationDenied,
    UnknownProfile,
    ActivationFailed,
    _Unknown,
}

impl From<TimeError> for Error {
    fn from(e: TimeError) -> Self {
        Error::Generic(e.into())
    }
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
    consecutive_timeouts: Cell<u8>,
    client: RefCell<C>,
}

impl<C: AtatClient> AtTx<C> {
    pub fn new(client: C, max_urc_attempts: u8) -> Self {
        Self {
            urc_attempts: Cell::new(0),
            consecutive_timeouts: Cell::new(0),
            max_urc_attempts,
            client: RefCell::new(client),
        }
    }

    pub fn clear_urc_queue(&self) -> Result<(), Error> {
        // self.client.try_borrow_mut()?.reset();
        while self.client.try_borrow_mut()?.check_urc::<Urc>().is_some() {}
        Ok(())
    }

    pub fn send<A: atat::AtatCmd>(&self, req: &A) -> Result<A::Response, Error> {
        self.client
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
                    if let atat::Error::Timeout = ate {
                        let new_value = self.consecutive_timeouts.get() + 1;
                        self.consecutive_timeouts.set(new_value);
                    }
                    Error::AT(ate)
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
            .map(|res| {
                self.consecutive_timeouts.set(0);
                res
            })
    }

    pub fn handle_urc<F: FnOnce(Urc) -> bool>(&self, f: F) -> Result<(), Error> {
        self.client
            .try_borrow_mut()?
            .peek_urc_with::<Urc, _>(|urc| {
                if !f(urc.clone()) {
                    let a = self.urc_attempts.get();
                    if a < self.max_urc_attempts {
                        self.urc_attempts.set(a + 1);
                        return false;
                    } else {
                        defmt::warn!(
                            "Dropping stale URC! {:?}",
                            defmt::Debug2Format::<consts::U256>(&urc)
                        );
                    }
                }
                self.urc_attempts.set(0);
                true
            });

        Ok(())
    }
}

pub struct Network<C, CLK>
where
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    pub(crate) status: RefCell<RegistrationState<CLK>>,
    pub(crate) context_state: Cell<ContextState>,
    pub(crate) at_tx: AtTx<C>,
}

impl<C, CLK> Network<C, CLK>
where
    C: AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    pub(crate) fn new(at_tx: AtTx<C>, timer: CLK) -> Self {
        Network {
            status: RefCell::new(RegistrationState::new(timer)),
            context_state: Cell::new(ContextState::Setup),
            at_tx,
        }
    }

    pub fn is_connected(&self) -> Result<bool, Error> {
        let ns = self.status.try_borrow()?;
        Ok(matches!(ns.conn_state, ConnectionState::Connected))
    }

    pub fn reset_reg_time(&self) -> Result<(), Error> {
        let mut ns = self.status.try_borrow_mut()?;
        let now = ns.timer.try_now().map_err(TimeError::from)?;

        ns.reg_start_time.replace(now);
        ns.reg_check_time = ns.reg_start_time;
        Ok(())
    }

    pub fn process_events(&self) -> Result<(), Error> {
        if self.at_tx.consecutive_timeouts.get() > 10 {
            defmt::warn!("Resetting the modem due to consecutive AT timeouts");
            return Err(Error::Generic(GenericError::Timeout));
        }

        self.handle_urc()?;
        self.check_registration_state()?;
        self.intervene_registration()?;
        // self.check_running_imsi();



        let mut ns = self.status.try_borrow_mut()?;

        let registration_check_interval = Seconds::<u32>(15);

        let now = ns.timer.try_now().map_err(TimeError::from)?;
        let should_check = ns
            .reg_check_time
            .and_then(|ref reg_check_time| {
                now.checked_duration_since(reg_check_time)
                    .and_then(|dur| dur.try_into().ok())
                    .map(|dur| dur >= registration_check_interval)
            })
            .unwrap_or(true);

        if ns.conn_state != ConnectionState::Connecting || !should_check {
            return Ok(());
        }

        ns.reg_check_time.replace(now);
        drop(ns);

        self.update_registration()?;

        let ns = self.status.try_borrow()?;

        let registration_timeout = Minutes::<u32>(5);

        let now = ns.timer.try_now().map_err(TimeError::from)?;
        let is_timeout = ns
            .reg_start_time
            .and_then(|ref reg_start_time| {
                now.checked_duration_since(reg_start_time)
                    .and_then(|dur| dur.try_into().ok())
                    .map(|dur| dur >= registration_timeout)
            })
            .unwrap_or(false);

        if ns.conn_state == ConnectionState::Connecting && is_timeout {
            defmt::warn!("Resetting the modem due to the network registration timeout");

            return Err(Error::Generic(GenericError::Timeout));
        }
        Ok(())
    }

    pub fn check_registration_state(&self) -> Result<(), Error> {
        let mut ns = self.status.try_borrow_mut()?;

        // Don't do anything if we are actually disconnected by choice
        if ns.conn_state == ConnectionState::Disconnected {
            return Ok(());
        }

        // If both (CSD + PSD) is registered, or EPS is registered, we are connected!
        if (ns.csd.registered() && ns.psd.registered()) || ns.eps.registered() {
            ns.set_connection_state(ConnectionState::Connected);
        } else if ns.conn_state == ConnectionState::Connected {
            // FIXME: potentially go back into connecting state only when getting into
            // a 'sticky' non-registered state
            ns.reset();
            ns.set_connection_state(ConnectionState::Connecting);
        }

        Ok(())
    }

    pub fn intervene_registration(&self) -> Result<(), Error> {
        let mut ns = self.status.try_borrow_mut()?;
        
        if ns.conn_state != ConnectionState::Connecting {
            return Ok(());
        }
        
        let timeout = Seconds(ns.registration_interventions * 15);
        
        let ts = ns.timer.try_now().map_err(TimeError::from)?;
        
        // If EPS has been sticky for longer than `timeout`
        if ns.eps.sticky() && ns.eps.duration(ts) >= timeout {
            // If (EPS + CSD) is not attempting registration
            if ns.eps.get_status() == registration::Status::NotRegistering
                && ns.csd.get_status() == registration::Status::NotRegistering
            {
                defmt::trace!(
                    "Sticky not registering state for {:?} s, PLMN reselection",
                    Seconds::<u32>::from(ns.eps.duration(ts)).integer()
                );

                ns.csd.reset();
                ns.psd.reset();
                ns.eps.reset();
                ns.registration_interventions += 1;
                self.send_internal(
                    &SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                    },
                    false,
                )
                .ok();
                return Ok(());

            // If (EPS + CSD) is denied registration
            } else if ns.eps.get_status() == registration::Status::Denied
                && ns.csd.get_status() == registration::Status::Denied
            {
                defmt::trace!(
                    "Sticky denied state for {:?} s, RF reset",
                    Seconds::<u32>::from(ns.eps.duration(ts)).integer()
                );
                ns.csd.reset();
                ns.psd.reset();
                ns.eps.reset();
                ns.registration_interventions += 1;
                self.send_internal(
                    &SetModuleFunctionality {
                        fun: Functionality::Minimum,
                        rst: None,
                    },
                    false,
                )?;
                self.send_internal(
                    &SetModuleFunctionality {
                        fun: Functionality::Full,
                        rst: None,
                    },
                    false,
                )?;
                return Ok(());
            }
        }

        // If CSD has been sticky for longer than `timeout`,
        // and (CSD + PSD) is denied registration.
        if ns.csd.sticky()
            && ns.csd.duration(ts) >= timeout
            && ns.csd.get_status() == registration::Status::Denied
            && ns.psd.get_status() == registration::Status::Denied
        {
            defmt::trace!(
                "Sticky CSD and PSD denied state for {:?} s, RF reset",
                Seconds::<u32>::from(ns.csd.duration(ts)).integer()
            );
            ns.csd.reset();
            ns.psd.reset();
            ns.eps.reset();
            ns.registration_interventions += 1;
            self.send_internal(
                &SetModuleFunctionality {
                    fun: Functionality::Minimum,
                    rst: None,
                },
                false,
            )?;
            self.send_internal(
                &SetModuleFunctionality {
                    fun: Functionality::Full,
                    rst: None,
                },
                false,
            )?;
            return Ok(());
        }

        // If CSD is registered, but PSD has been sticky for longer than `timeout`,
        // and (PSD + EPS) is not attempting registration.
        if ns.csd.registered()
            && ns.psd.sticky()
            && ns.psd.duration(ts) >= timeout
            && ns.psd.get_status() == registration::Status::NotRegistering
            && ns.eps.get_status() == registration::Status::NotRegistering
        {
            defmt::trace!(
                "Sticky not registering PSD state for {:?} s, force GPRS attach",
                Seconds::<u32>::from(ns.psd.duration(ts)).integer()
            );
            ns.psd.reset();
            ns.registration_interventions += 1;
            self.send_internal(&GetPDPContextState, true)?;

            if self
                .send_internal(
                    &SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: None,
                    },
                    true,
                )
                .is_err()
            {
                ns.csd.reset();
                ns.psd.reset();
                ns.eps.reset();
                defmt::trace!("GPRS attach failed, try PLMN reselection");
                self.send_internal(
                    &SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                    },
                    true,
                )?;
            }
        }

        Ok(())
    }

    pub fn update_registration(&self) -> Result<(), Error> {
        let mut status = self.status.try_borrow_mut()?;
        let ts = status.timer.try_now().map_err(TimeError::from)?;

        if let Ok(reg) = self.send_internal(&GetNetworkRegistrationStatus, false) {
            status.compare_and_set(reg.into(), ts);
        }

        if let Ok(reg) = self.send_internal(&GetGPRSNetworkRegistrationStatus, false) {
            status.compare_and_set(reg.into(), ts);
        }

        if let Ok(reg) = self.send_internal(&GetEPSNetworkRegistrationStatus, false) {
            status.compare_and_set(reg.into(), ts);
        }

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
                    if let Ok(mut params) = self.status.try_borrow_mut() {
                        if let Ok(ts) = params.timer.try_now() {
                            params.compare_and_set(reg_params.into(), ts)
                        }
                    }
                }
                Urc::EPSNetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.status.try_borrow_mut() {
                        if let Ok(ts) = params.timer.try_now() {
                            params.compare_and_set(reg_params.into(), ts)
                        }
                    }
                }
                Urc::NetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.status.try_borrow_mut() {
                        if let Ok(ts) = params.timer.try_now() {
                            params.compare_and_set(reg_params.into(), ts)
                        }
                    }
                }
                Urc::DataConnectionActivated(psn::urc::DataConnectionActivated {
                    result,
                    ip_addr: _,
                }) => {
                    defmt::info!("[URC] DataConnectionActivated {:u8}", result);
                    self.context_state.set(ContextState::Active);
                }
                Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    profile_id,
                }) => {
                    defmt::info!("[URC] DataConnectionDeactivated {:?}", profile_id);
                    self.context_state.set(ContextState::Activating);
                }
                Urc::MessageWaitingIndication(_) => {
                    defmt::info!("[URC] MessageWaitingIndication");
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
                defmt::error!(
                    "Failed handle URC  {:?}",
                    defmt::Debug2Format::<consts::U64>(&e)
                );
            }
        }

        self.at_tx.send(req)
    }
}

#[cfg(test)]
mod tests {
    use embedded_time::{duration::*, Instant};

    use crate::{
        registration::Status,
        test_helpers::{MockAtClient, MockTimer},
    };

    use super::*;

    #[test]
    fn intervene_registration() {
        // Setup
        let tx = AtTx::new(MockAtClient::new(0), 5);
        let timer = MockTimer::new(Some(25_234));
        let network = Network::new(tx, timer);
        let mut ns = network.status.borrow_mut();
        ns.conn_state = ConnectionState::Connecting;
        // Update both started & updated
        ns.eps
            .set_status(Status::NotRegistering, Instant::new(1234));
        // Update only updated
        ns.eps
            .set_status(Status::NotRegistering, Instant::new(1534));
        ns.csd
            .set_status(Status::NotRegistering, Instant::new(1534));

        assert_eq!(ns.eps.updated(), Some(Instant::new(1534)));
        assert_eq!(ns.eps.started(), Some(Instant::new(1234)));
        assert!(ns.eps.sticky());

        let ts = ns.timer.try_now().unwrap();
        assert_eq!(ns.eps.duration(ts), Milliseconds(24_000_u32));
        drop(ns);

        assert!(network.intervene_registration().is_ok());

        let ns = network.status.borrow();
        assert_eq!(ns.registration_interventions, 2);
    }

    #[test]
    fn reset_reg_time() {
        let tx = AtTx::new(MockAtClient::new(0), 5);
        let timer = MockTimer::new(Some(1234));
        let network = Network::new(tx, timer);

        assert!(network.reset_reg_time().is_ok());

        let ns = network.status.borrow();
        assert_eq!(ns.reg_start_time, Some(Instant::new(1234)));
        assert_eq!(ns.reg_check_time, Some(Instant::new(1234)));
    }

    #[test]
    fn check_registration_state() {
        let tx = AtTx::new(MockAtClient::new(0), 5);
        let timer = MockTimer::new(Some(1234));
        let network = Network::new(tx, timer);

        // Check that `ConnectionState` will change from `Connected` to `Connecting`
        // with a state reset, if neither (csd + psd) || eps is actually registered
        let mut ns = network.status.borrow_mut();
        ns.conn_state = ConnectionState::Connected;
        ns.registration_interventions = 3;
        ns.csd.set_status(Status::Denied, Instant::new(1));
        ns.eps.set_status(Status::NotRegistering, Instant::new(5));
        drop(ns);

        assert!(network.check_registration_state().is_ok());

        let mut ns = network.status.borrow_mut();
        assert_eq!(ns.conn_state, ConnectionState::Connecting);
        assert_eq!(ns.reg_start_time, Some(Instant::new(1234)));
        assert_eq!(ns.reg_check_time, Some(Instant::new(1234)));
        assert_eq!(ns.csd.get_status(), Status::None);
        assert_eq!(ns.csd.updated(), None);
        assert_eq!(ns.csd.started(), None);
        assert_eq!(ns.psd.get_status(), Status::None);
        assert_eq!(ns.psd.updated(), None);
        assert_eq!(ns.psd.started(), None);
        assert_eq!(ns.eps.get_status(), Status::None);
        assert_eq!(ns.eps.updated(), None);
        assert_eq!(ns.eps.started(), None);

        // Check that `ConnectionState` will change from `Connecting` to `Connected`
        // if eps is actually registered
        ns.eps.set_status(Status::Roaming, Instant::new(5));
        drop(ns);

        assert!(network.check_registration_state().is_ok());

        let mut ns = network.status.borrow_mut();
        assert_eq!(ns.conn_state, ConnectionState::Connected);

        // Check that `ConnectionState` will change from `Connecting` to `Connected`
        // if (csd + psd) is actually registered
        ns.conn_state = ConnectionState::Connecting;
        ns.reset();
        ns.eps.set_status(Status::Denied, Instant::new(5));
        ns.csd.set_status(Status::Roaming, Instant::new(5));
        ns.psd.set_status(Status::Home, Instant::new(5));
        drop(ns);

        assert!(network.check_registration_state().is_ok());

        let ns = network.status.borrow_mut();
        assert_eq!(ns.conn_state, ConnectionState::Connected);
    }

    #[test]
    fn unhandled_urcs() {
        let tx = AtTx::new(MockAtClient::new(0), 5);

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
