use crate::{
    command::ip_transport_layer,
    command::psn::GetEPSNetworkRegistrationStatus,
    command::psn::GetGPRSNetworkRegistrationStatus,
    command::psn::SetPacketSwitchedEventReporting,
    command::{network_service, psn, Urc},
    error::GenericError,
    state::Event,
    state::Registration,
    state::RegistrationStatus,
    APNInfo,
};
use atat::{atat_derive::AtatLen, AtatClient};
use core::cell::{BorrowError, BorrowMutError, Cell, RefCell};
use embedded_nal::IpAddr;
use hash32_derive::Hash32;
use heapless::{consts, FnvIndexMap, IndexMap};
use network_service::{types::OperatorSelectionMode, GetOperatorSelection, SetOperatorSelection};
use psn::{
    responses::GPRSAttached, types::GPRSAttachedState, types::PSEventReportingMode,
    GetGPRSAttached, SetGPRSAttached,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, defmt::Format)]
pub enum Error {
    Generic(GenericError),
    AT(atat::Error),
    RegistrationDenied,
    UnknownProfile,
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

#[derive(Debug, Clone)]
pub enum ProfileState {
    Unknown,
    Deactivated,
    Activating(ContextId, APNInfo),
    Active(ContextId, Option<IpAddr>),
}

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
                    }
                }
                self.urc_attempts.set(0);
                true
            });

        Ok(())
    }
}

pub struct Network<C> {
    pub(crate) registration: RefCell<Registration>,
    pub(crate) pdp_context_active: Cell<bool>,
    pub(crate) attached: Cell<bool>,
    // NOTE: Currently only a single profile is supported at a time!
    pub(crate) profile_status: RefCell<FnvIndexMap<ProfileId, ProfileState, consts::U1>>,
    pub(crate) at_tx: AtTx<C>,
}

impl<C> Network<C>
where
    C: AtatClient,
{
    pub(crate) fn new(at_tx: AtTx<C>) -> Self {
        let mut profile_status = IndexMap::new();
        for i in 0..profile_status.capacity() as u8 {
            profile_status
                .insert(ProfileId(i), ProfileState::Unknown)
                .ok();
        }
        Network {
            registration: RefCell::new(Registration::default()),
            profile_status: RefCell::new(profile_status),
            attached: Cell::new(false),
            pdp_context_active: Cell::new(false),
            at_tx,
        }
    }

    pub fn get_event(&self) -> Result<Option<Event>, Error> {
        Ok(self.registration.try_borrow_mut()?.events.pop())
    }

    pub fn push_event(&self, event: Event) -> Result<(), Error> {
        self.registration.try_borrow_mut()?.push_event(event);
        Ok(())
    }

    pub fn clear_events(&self) -> Result<(), Error> {
        self.registration.try_borrow_mut()?.events.clear();
        Ok(())
    }

    pub fn context_active(&self, profile_id: ProfileId, cid: ContextId) -> Result<bool, Error> {
        Ok(
            matches!(self.get_profile_state(profile_id)?, ProfileState::Active(active_cid, _) if active_cid == cid),
        )
    }

    pub fn finish_activating_profile_state(&self, ip_addr: Option<IpAddr>) -> Result<(), Error> {
        let (profile_id, cid) = self
            .profile_status
            .try_borrow_mut()?
            .iter()
            .find_map(|(profile_id, prev_state)| {
                if let ProfileState::Activating(cid, _) = prev_state {
                    Some((*profile_id, *cid))
                } else {
                    None
                }
            })
            .ok_or(Error::UnknownProfile)?;

        self.set_profile_state(profile_id, ProfileState::Active(cid, ip_addr))
    }

    pub fn get_profile_state(&self, profile_id: ProfileId) -> Result<ProfileState, Error> {
        match self.profile_status.try_borrow()?.get(&profile_id) {
            Some(state) => Ok(state.clone()),
            None => Err(Error::UnknownProfile),
        }
    }

    pub fn set_profile_state(
        &self,
        profile_id: ProfileId,
        state: ProfileState,
    ) -> Result<(), Error> {
        if let Some(v) = self.profile_status.try_borrow_mut()?.get_mut(&profile_id) {
            *v = state;
            Ok(())
        } else {
            Err(Error::UnknownProfile)
        }
    }

    pub fn is_registered(&self) -> Result<Option<RegistrationStatus>, Error> {
        // accept only CGREG/CEREG. CREG is for circuit switch network changed. If we accept CREG attach will fail if also
        // CGREG/CEREG is not registered.
        let mut status = self.registration.try_borrow_mut()?;
        status.set_params(
            self.send_internal(&GetEPSNetworkRegistrationStatus, true)?
                .into(),
        );

        if status.is_registered().is_none() {
            status.set_params(
                self.send_internal(&GetGPRSNetworkRegistrationStatus, true)?
                    .into(),
            );
        }

        // TODO:
        // in manual registering we are forcing registration to certain network so we don't accept active context or attached
        // as indication that device is registered to correct network.
        // if plmn.is_some() {
        //     return self.is_registered();
        // }

        if let Some(registration_status) = status.is_registered() {
            Ok(Some(registration_status))
        } else if self.attached.get() || self.pdp_context_active.get() {
            Ok(Some(RegistrationStatus::AlreadyRegistered))
        } else {
            Ok(None)
        }
    }

    pub fn set_registration(&self, plmn: Option<&str>) -> nb::Result<(), Error> {
        match plmn {
            Some(p) => {
                defmt::debug!("Manual network registration to {:str}", p);
                // FIXME:
                // https://github.com/ARMmbed/mbed-os/blob/master/connectivity/cellular/source/framework/AT/AT_CellularNetwork.cpp#L227
                //
                // self.send_internal(&SetOperatorSelection {mode:
                //     OperatorSelectionMode::Manual,
                //     },
                //     true,
                // )?;
                unimplemented!();
            }
            None => {
                match self.send_internal(&GetOperatorSelection, true)?.mode {
                    OperatorSelectionMode::Automatic => {}
                    _ => {
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
        }
    }

    pub fn register(&self, plmn: Option<&str>) -> nb::Result<(), Error> {
        if let Some(registration_status) = self.is_registered()? {
            if registration_status == RegistrationStatus::AlreadyRegistered {
                defmt::info!("Already registered!");
            }
            return Ok(());
        }

        if self
            .registration
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?
            .is_denied()
        {
            return Err(nb::Error::Other(Error::RegistrationDenied));
        }

        self.set_registration(plmn)?;

        Err(nb::Error::WouldBlock)
    }

    pub fn attach(&self) -> Result<(), Error> {
        if !self.attached.get() {
            let GPRSAttached { state } = self.send_internal(&GetGPRSAttached, true)?;

            if state != GPRSAttachedState::Attached {
                defmt::debug!("Network attach");
                self.send_internal(
                    &SetGPRSAttached {
                        state: GPRSAttachedState::Attached,
                    },
                    true,
                )?;
            }
        }
        Ok(())
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
                    self.attach().ok();
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
                    if let Ok(mut params) = self.registration.try_borrow_mut() {
                        params.compare_and_set(reg_params.into())
                    }
                }
                Urc::EPSNetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.registration.try_borrow_mut() {
                        params.compare_and_set(reg_params.into())
                    }
                }
                Urc::NetworkRegistration(reg_params) => {
                    if let Ok(mut params) = self.registration.try_borrow_mut() {
                        params.compare_and_set(reg_params.into())
                    }
                }
                Urc::DataConnectionActivated(psn::urc::DataConnectionActivated {
                    result,
                    ip_addr,
                }) => {
                    defmt::info!("[URC] DataConnectionActivated {:u8}", result);
                    if result == 0 {
                        self.finish_activating_profile_state(ip_addr).ok();
                    }
                }
                Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    profile_id,
                }) => {
                    defmt::info!("[URC] DataConnectionDeactivated {:?}", profile_id);
                    // Set the state of `profile_id`!
                    self.set_profile_state(profile_id, ProfileState::Deactivated)
                        .ok();
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
