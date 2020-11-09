use crate::{APNInfo, command::{network_service, psn, Urc}, error::GenericError, state::{RANStatus, RadioAccessNetwork}};
use atat::{atat_derive::AtatLen, AtatClient};
use core::cell::{BorrowError, BorrowMutError, Cell, RefCell};
use hash32_derive::Hash32;
use heapless::{consts, FnvIndexMap, IndexMap};
use network_service::{types::OperatorSelectionMode, GetOperatorSelection, SetOperatorSelection};
use psn::{
    responses::{EPSNetworkRegistrationStatus, GPRSAttached, GPRSNetworkRegistrationStatus},
    types::GPRSAttachedState,
    GetEPSNetworkRegistrationStatus, GetGPRSAttached, GetGPRSNetworkRegistrationStatus,
    SetGPRSAttached,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, defmt::Format)]
pub enum Error {
    Generic(GenericError),
    RegistrationDenied,
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
    Active(ContextId),
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
    pub(crate) ran_status: RefCell<RANStatus>,
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
            ran_status: RefCell::new(RANStatus::new()),
            profile_status: RefCell::new(profile_status),
            at_tx,
        }
    }

    pub fn context_active(&self, profile_id: ProfileId, cid: ContextId) -> Result<bool, Error> {
        if let Some(state) = self.profile_status.try_borrow()?.get(&profile_id) {
            Ok(if let ProfileState::Active(active_cid) = state {
                active_cid == &cid
            } else {
                false
            })
        } else {
            Err(Error::_Unknown)
        }
    }

    pub fn finish_activating_profile_state(&self) -> Result<(), Error> {
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
            .ok_or(Error::_Unknown)?;

        self.set_profile_state(profile_id, ProfileState::Active(cid))
    }

    pub fn get_profile_state(&self, profile_id: ProfileId) -> Result<ProfileState, Error> {
        match self.profile_status.try_borrow()?.get(&profile_id) {
            Some(state) => Ok(state.clone()),
            None => Err(Error::_Unknown),
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
            defmt::error!("ProfileStatus map does not contain {:?}!", profile_id);
            Err(Error::_Unknown)
        }
    }

    pub fn is_registered(&self) -> Result<bool, Error> {
        Ok(self.ran_status.try_borrow()?.is_registered())
    }

    pub fn register(&self, plmn: Option<&str>) -> nb::Result<(), Error> {
        if self.is_registered()? {
            return Ok(());
        }

        match plmn {
            Some(p) => {
                defmt::debug!("Manual network registration to {:str}", p);
                unimplemented!();
                // FIXME:
                // https://github.com/ARMmbed/mbed-os/blob/master/connectivity/cellular/source/framework/AT/AT_CellularNetwork.cpp#L227
                //
                // self.send_internal(&SetOperatorSelection {mode:
                //     OperatorSelectionMode::Manual,
                //     },
                //     true,
                // )?;
            }
            None => {
                defmt::debug!("Automatic network registration");
                let cops = self.send_internal(&GetOperatorSelection, true)?;

                // FIXME: Is this correct?
                // if cops.act.is_some() {
                //     return Ok(());
                // }

                match cops.mode {
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
            }
        }

        let GPRSNetworkRegistrationStatus { stat, .. } =
            self.send_internal(&GetGPRSNetworkRegistrationStatus, true)?;
        if let Ok(mut status) = self.ran_status.try_borrow_mut() {
            status.set(RadioAccessNetwork::Utran, stat.into());
        }

        let EPSNetworkRegistrationStatus { stat, .. } =
            self.send_internal(&GetEPSNetworkRegistrationStatus, true)?;
        if let Ok(mut status) = self.ran_status.try_borrow_mut() {
            status.set(RadioAccessNetwork::Eutran, stat.into());
        }

        if self.is_registered()? {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    pub fn attach(&self) -> Result<(), Error> {
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
        Ok(())
    }

    pub(crate) fn handle_urc(&self) -> Result<(), Error> {
        self.at_tx.handle_urc(|urc| {
            match urc {
                Urc::ExtendedPSNetworkRegistration(psn::urc::ExtendedPSNetworkRegistration {
                    state,
                }) => {
                    defmt::info!("[URC] ExtendedPSNetworkRegistration {:?}", state);
                }
                Urc::GPRSNetworkRegistration(psn::urc::GPRSNetworkRegistration { stat }) => {
                    defmt::info!("[URC] GPRSNetworkRegistration {:?}", stat);
                    if let Ok(mut status) = self.ran_status.try_borrow_mut() {
                        status.set(RadioAccessNetwork::Utran, stat.into());
                    }
                }
                Urc::EPSNetworkRegistration(psn::urc::EPSNetworkRegistration { stat }) => {
                    defmt::info!("[URC] EPSNetworkRegistration {:?}", stat);
                    if let Ok(mut status) = self.ran_status.try_borrow_mut() {
                        status.set(RadioAccessNetwork::Eutran, stat.into());
                    }
                }
                Urc::NetworkRegistration(network_service::urc::NetworkRegistration { stat }) => {
                    defmt::info!("[URC] NetworkRegistration {:?}", stat);
                    if let Ok(mut status) = self.ran_status.try_borrow_mut() {
                        status.set(RadioAccessNetwork::Geran, stat.into());
                    }
                }
                Urc::DataConnectionActivated(psn::urc::DataConnectionActivated {
                    result,
                    ip_addr: _,
                }) => {
                    defmt::info!("[URC] DataConnectionActivated {:u8}", result);
                    if result == 0 {
                        self.finish_activating_profile_state().ok();
                    }
                }
                Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    profile_id,
                }) => {
                    defmt::info!("[URC] DataConnectionDeactivated {:?}", profile_id);
                    // Set the state of `profile_id`!
                    self
                        .set_profile_state(profile_id, ProfileState::Deactivated)
                        .ok();
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
                            "{:?}:",
                            ate,
                            // core::convert::AsRef::<[u8]>::as_ref(&req.as_bytes())
                        ),
                    };
                    // ate.into()
                    Error::_Unknown
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }
}
