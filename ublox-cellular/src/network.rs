use crate::{
    client::{URC_CAPACITY, URC_SUBSCRIBERS},
    command::{
        general::GetCIMI,
        mobile_control::{
            types::{Functionality, ResetMode},
            GetExtendedErrorReport, SetModuleFunctionality,
        },
        network_service::{
            types::OperatorSelectionMode, GetNetworkRegistrationStatus, SetOperatorSelection,
        },
        psn::{
            self, types::PDPContextStatus, GetEPSNetworkRegistrationStatus,
            GetGPRSNetworkRegistrationStatus, GetPDPContextState, SetPDPContextState,
        },
        Urc, AT,
    },
    error::GenericError,
    registration::{self, ConnectionState, RegistrationState},
    services::data::{ContextState, PROFILE_ID},
};
use atat::{atat_derive::AtatLen, blocking::AtatClient, UrcSubscription};
use embassy_time::{Duration, Instant};
use hash32_derive::Hash32;
use serde::{Deserialize, Serialize};

const REGISTRATION_CHECK_INTERVAL: Duration = Duration::from_secs(15);
const REGISTRATION_TIMEOUT: Duration = Duration::from_secs(3 * 60);
const CHECK_IMSI_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Generic(GenericError),
    AT(atat::Error),
    RegistrationDenied,
    UnknownProfile,
    ActivationFailed,
    _Unknown,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash32, Serialize, Deserialize, AtatLen)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ProfileId(pub u8);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, AtatLen)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ContextId(pub u8);

pub struct AtTx<'sub, AtCl> {
    consecutive_timeouts: u8,
    urc_subscription: UrcSubscription<'sub, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    client: AtCl,
}

impl<'sub, AtCl: AtatClient> AtTx<'sub, AtCl> {
    pub fn new(
        client: AtCl,
        urc_subscription: UrcSubscription<'sub, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    ) -> Self {
        Self {
            consecutive_timeouts: 0,
            urc_subscription,
            client,
        }
    }

    pub fn send_ignore_timeout<A, const LEN: usize>(
        &mut self,
        req: &A,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
    {
        self.client
            .send_retry(req)
            .map_err(|e| match e {
                atat::Error::Timeout => {
                    self.consecutive_timeouts =
                        self.consecutive_timeouts.saturating_add(A::ATTEMPTS);
                    Error::AT(atat::Error::Timeout)
                }
                atat::Error::Read => Error::AT(atat::Error::Read),
                atat::Error::Write => Error::AT(atat::Error::Write),
                atat::Error::InvalidResponse => Error::AT(atat::Error::InvalidResponse),
                atat::Error::Aborted => Error::AT(atat::Error::Aborted),
                atat::Error::Parse => Error::AT(atat::Error::Parse),
                _ => Error::AT(atat::Error::Error),
            })
            .map(|res| {
                self.consecutive_timeouts = 0;
                res
            })
    }

    pub fn send<A, const LEN: usize>(&mut self, req: &A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
    {
        self.client
            .send_retry(req)
            .map_err(|e| match e {
                atat::Error::Timeout => {
                    self.consecutive_timeouts =
                        self.consecutive_timeouts.saturating_add(A::ATTEMPTS);
                    Error::AT(atat::Error::Timeout)
                }
                atat::Error::Read => Error::AT(atat::Error::Read),
                atat::Error::Write => Error::AT(atat::Error::Write),
                atat::Error::InvalidResponse => Error::AT(atat::Error::InvalidResponse),
                atat::Error::Aborted => Error::AT(atat::Error::Aborted),
                atat::Error::Parse => Error::AT(atat::Error::Parse),
                _ => Error::AT(atat::Error::Error),
            })
            .map(|res| {
                self.consecutive_timeouts = 0;
                res
            })
    }

    pub fn handle_urc<F: FnOnce(Urc) -> bool>(&mut self, f: F) -> Result<(), Error> {
        if let Some(urc) = self.urc_subscription.try_next_message_pure() {
            f(urc);
        }
        Ok(())
    }
}

pub struct Network<'sub, AtCl> {
    pub(crate) status: RegistrationState,
    pub(crate) context_state: ContextState,
    pub(crate) at_tx: AtTx<'sub, AtCl>,
}

impl<'sub, AtCl> Network<'sub, AtCl>
where
    AtCl: AtatClient,
{
    pub(crate) fn new(at_tx: AtTx<'sub, AtCl>) -> Self {
        Self {
            status: RegistrationState::new(),
            context_state: ContextState::Setup,
            at_tx,
        }
    }

    pub fn is_connected(&self) -> Result<bool, Error> {
        Ok(matches!(self.status.conn_state, ConnectionState::Connected))
    }

    pub fn reset_reg_time(&mut self) -> Result<(), Error> {
        self.status.reg_start_time.replace(Instant::now());
        self.status.reg_check_time = self.status.reg_start_time;
        Ok(())
    }

    pub fn process_events(&mut self) -> Result<(), Error> {
        if self.at_tx.consecutive_timeouts > 10 {
            self.at_tx.consecutive_timeouts = 0;
            warn!("Resetting the modem due to consecutive AT timeouts");
            return Err(Error::Generic(GenericError::Timeout));
        }

        self.handle_urc().ok(); // Ignore errors
        self.check_registration_state();
        self.intervene_registration()?;
        self.check_running_imsi().ok(); // Ignore errors

        let now = Instant::now();
        let should_check = self
            .status
            .reg_check_time
            .and_then(|reg_check_time| {
                now.checked_duration_since(reg_check_time)
                    .map(|dur| dur >= REGISTRATION_CHECK_INTERVAL)
            })
            .unwrap_or(true);

        if !should_check {
            return Ok(());
        }

        self.status.reg_check_time.replace(now);

        self.update_registration()?;

        let now = Instant::now();
        let is_timeout = self
            .status
            .reg_start_time
            .and_then(|reg_start_time| {
                now.checked_duration_since(reg_start_time)
                    .map(|dur| dur >= REGISTRATION_TIMEOUT)
            })
            .unwrap_or(false);

        if self.status.conn_state == ConnectionState::Connecting && is_timeout {
            warn!("Resetting the modem due to the network registration timeout");

            return Err(Error::Generic(GenericError::Timeout));
        }
        Ok(())
    }

    pub fn check_running_imsi(&mut self) -> Result<(), Error> {
        // Check current IMSI if registered successfully in which case
        // imsi_check_time will be `None`, else if not registered, check after
        // CHECK_IMSI_TIMEOUT is expired
        let now = Instant::now();
        let check_imsi = self
            .status
            .imsi_check_time
            .and_then(|imsi_check_time| {
                now.checked_duration_since(imsi_check_time)
                    .map(|dur| dur >= CHECK_IMSI_TIMEOUT)
            })
            .unwrap_or(true);

        if check_imsi {
            // NOTE: The CIMI command has been known to not have an immediate response on u-blox modems
            // and currently has a 10 second timeout.  This command is also only for
            // logging purposes to monitor the currently selected IMSI on EtherSIM
            // during registration.  For these reasons we are intentionally not
            // registering a parserError_ when this command does not return
            // AtResponse::OK.  Instead, in the case of a non-OK response, we will
            // follow up the command with an AT/OK check and subsequent
            // checkParser() call to catch/address any modem parsing issues.
            match self.send_internal(&GetCIMI, false) {
                Ok(_) => {}
                Err(_) => {
                    self.send_internal(&AT, false)?;
                }
            }

            self.status.imsi_check_time.replace(now);
        }
        Ok(())
    }

    pub fn check_registration_state(&mut self) {
        // Don't do anything if we are actually disconnected by choice
        if self.status.conn_state == ConnectionState::Disconnected {
            return;
        }

        // If both (CSD + PSD) is registered, or EPS is registered, we are connected!
        if (self.status.csd.registered() && self.status.psd.registered())
            || self.status.eps.registered()
        {
            self.status.set_connection_state(ConnectionState::Connected);
        } else if self.status.conn_state == ConnectionState::Connected {
            // FIXME: potentially go back into connecting state only when getting into
            // a 'sticky' non-registered state
            self.status.reset();
            self.status
                .set_connection_state(ConnectionState::Connecting);
        }
    }

    pub fn intervene_registration(&mut self) -> Result<(), Error> {
        if self.status.conn_state != ConnectionState::Connecting {
            return Ok(());
        }

        let now = Instant::now();

        // If EPS has been sticky for longer than `timeout`
        let timeout = Duration::from_secs(self.status.registration_interventions as u64 * 15);
        if self.status.eps.sticky() && self.status.eps.duration(now) >= timeout {
            // If (EPS + CSD) is not attempting registration
            if self.status.eps.get_status() == registration::Status::NotRegistering
                && self.status.csd.get_status() == registration::Status::NotRegistering
            {
                debug!(
                    "Sticky not registering state for {}, PLMN reselection",
                    self.status.eps.duration(now)
                );

                self.status.csd.reset();
                self.status.psd.reset();
                self.status.eps.reset();
                self.status.registration_interventions =
                    self.status.registration_interventions.saturating_add(1);

                self.send_internal(
                    &SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                        format: Some(2),
                    },
                    false,
                )
                .ok(); // Ignore result
                return Ok(());

            // If (EPS + CSD) is denied registration
            } else if self.status.eps.get_status() == registration::Status::Denied
                && self.status.csd.get_status() == registration::Status::Denied
            {
                debug!(
                    "Sticky denied state for {}, RF reset",
                    self.status.eps.duration(now)
                );
                self.status.csd.reset();
                self.status.psd.reset();
                self.status.eps.reset();
                self.status.registration_interventions =
                    self.status.registration_interventions.saturating_add(1);
                self.send_internal(
                    &SetModuleFunctionality {
                        fun: Functionality::Minimum,
                        // SARA-R5 This parameter can be used only when <fun> is 1, 4 or 19
                        #[cfg(feature = "sara-r5")]
                        rst: None,
                        #[cfg(not(feature = "sara-r5"))]
                        rst: Some(ResetMode::DontReset),
                    },
                    false,
                )?;
                self.send_internal(
                    &SetModuleFunctionality {
                        fun: Functionality::Full,
                        rst: Some(ResetMode::DontReset),
                    },
                    false,
                )?;
                return Ok(());
            }
        }

        // If CSD has been sticky for longer than `timeout`,
        // and (CSD + PSD) is denied registration.
        if self.status.csd.sticky()
            && self.status.csd.duration(now) >= timeout
            && matches!(
                self.status.csd.get_status(),
                registration::Status::Denied | registration::Status::Roaming
            )
            && self.status.psd.get_status() == registration::Status::Denied
        {
            debug!(
                "Sticky CSD and PSD denied state for {}, RF reset",
                self.status.csd.duration(now)
            );
            self.status.csd.reset();
            self.status.psd.reset();
            self.status.eps.reset();
            self.status.registration_interventions =
                self.status.registration_interventions.saturating_add(1);
            self.send_internal(
                &SetModuleFunctionality {
                    fun: Functionality::Minimum,
                    // SARA-R5 This parameter can be used only when <fun> is 1, 4 or 19
                    #[cfg(feature = "sara-r5")]
                    rst: None,
                    #[cfg(not(feature = "sara-r5"))]
                    rst: Some(ResetMode::DontReset),
                },
                false,
            )?;
            self.send_internal(
                &SetModuleFunctionality {
                    fun: Functionality::Full,
                    rst: Some(ResetMode::DontReset),
                },
                false,
            )?;
            return Ok(());
        }

        // If CSD is registered, but PSD has been sticky for longer than `timeout`,
        // and (PSD + EPS) is not attempting registration.
        if self.status.csd.registered()
            && self.status.psd.sticky()
            && self.status.psd.duration(now) >= timeout
            && self.status.psd.get_status() == registration::Status::NotRegistering
            && self.status.eps.get_status() == registration::Status::NotRegistering
        {
            debug!(
                "Sticky not registering PSD state for {}, force GPRS attach",
                self.status.psd.duration(now)
            );
            self.status.psd.reset();
            self.status.registration_interventions =
                self.status.registration_interventions.saturating_add(1);
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
                self.status.csd.reset();
                self.status.psd.reset();
                self.status.eps.reset();
                warn!("GPRS attach failed, try PLMN reselection");
                self.send_internal(
                    &SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                        format: Some(2),
                    },
                    true,
                )?;
            }
        }

        Ok(())
    }

    pub fn update_registration(&mut self) -> Result<(), Error> {
        self.send_internal(&GetExtendedErrorReport, false).ok();

        if let Ok(reg) = self.send_internal(&GetNetworkRegistrationStatus, false) {
            self.status.compare_and_set(reg.into());
        }

        if let Ok(reg) = self.send_internal(&GetGPRSNetworkRegistrationStatus, false) {
            self.status.compare_and_set(reg.into());
        }

        if let Ok(reg) = self.send_internal(&GetEPSNetworkRegistrationStatus, false) {
            self.status.compare_and_set(reg.into());
        }

        Ok(())
    }

    pub(crate) fn handle_urc(&mut self) -> Result<(), Error> {
        // TODO: How to do this cleaner?
        let mut ctx_state = self.context_state;
        // let mut new_reg_params: Option<RegistrationParams> = None;

        self.at_tx.handle_urc(|urc| {
            match urc {
                Urc::NetworkDetach => {
                    warn!("Network Detach URC!");
                }
                Urc::MobileStationDetach => {
                    warn!("ME Detach URC!");
                }
                Urc::NetworkDeactivate => {
                    warn!("Network Deactivate URC!");
                }
                Urc::MobileStationDeactivate => {
                    warn!("ME Deactivate URC!");
                }
                Urc::NetworkPDNDeactivate => {
                    warn!("Network PDN Deactivate URC!");
                }
                Urc::MobileStationPDNDeactivate => {
                    warn!("ME PDN Deactivate URC!");
                }
                Urc::ExtendedPSNetworkRegistration(psn::urc::ExtendedPSNetworkRegistration {
                    state,
                }) => {
                    info!("[URC] ExtendedPSNetworkRegistration {:?}", state);
                }
                // FIXME: Currently `atat` is unable to distinguish `xREG` family of
                // commands from URC's

                // Urc::GPRSNetworkRegistration(reg_params) => {
                //     new_reg_params.replace(reg_params.into());
                // }
                // Urc::EPSNetworkRegistration(reg_params) => {
                //     new_reg_params.replace(reg_params.into());
                // }
                // Urc::NetworkRegistration(reg_params) => {
                //     new_reg_params.replace(reg_params.into());
                // }
                Urc::DataConnectionActivated(psn::urc::DataConnectionActivated {
                    result,
                    ip_addr: _,
                }) => {
                    info!("[URC] DataConnectionActivated {}", result);
                    if result == 0 {
                        ctx_state = ContextState::Active;
                    } else {
                        ctx_state = ContextState::Setup;
                    }
                }
                Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    profile_id,
                }) => {
                    info!("[URC] DataConnectionDeactivated {:?}", profile_id);
                    if profile_id == PROFILE_ID {
                        ctx_state = ContextState::Activating;
                    }
                }
                Urc::MessageWaitingIndication(_) => {
                    info!("[URC] MessageWaitingIndication");
                }
                _ => return false,
            };
            true
        })?;

        // if let Some(reg_params) = new_reg_params {
        //     self.status.compare_and_set(reg_params)
        // }

        self.context_state = ctx_state;
        Ok(())
    }

    pub(crate) fn send_internal<A, const LEN: usize>(
        &mut self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
    {
        if check_urc {
            if let Err(e) = self.handle_urc() {
                error!("Failed handle URC {:?}", &e);
            }
        }

        self.at_tx.send(req)
    }
}

#[cfg(test)]
mod tests {
    use embassy_time::Instant;

    use super::*;
    use crate::{registration::Status, test_helpers::MockAtClient};

    // #[test]
    // fn reset_reg_time() {
    //     let tx = AtTx::new(MockAtClient::new(0), 5);
    //     let mut network = Network::new(tx);

    //     assert!(network.reset_reg_time().is_ok());

    //     assert_eq!(
    //         network.status.reg_start_time,
    //         Some(Instant::from_ticks(1234))
    //     );
    //     assert_eq!(
    //         network.status.reg_check_time,
    //         Some(Instant::from_ticks(1234))
    //     );
    // }

    // #[test]
    // fn check_registration_state() {
    //     let tx = AtTx::new(MockAtClient::new(0), 5);
    //     let mut network = Network::new(tx);

    //     // Check that `ConnectionState` will change from `Connected` to `Connecting`
    //     // with a state reset, if neither (csd + psd) || eps is actually registered
    //     network.status.conn_state = ConnectionState::Connected;
    //     network.status.registration_interventions = 3;
    //     network.status.csd.set_status(Status::Denied);
    //     network.status.eps.set_status(Status::NotRegistering);

    //     network.check_registration_state();

    //     assert_eq!(network.status.conn_state, ConnectionState::Connecting);
    //     assert_eq!(
    //         network.status.reg_start_time,
    //         Some(Instant::from_ticks(1234))
    //     );
    //     assert_eq!(
    //         network.status.reg_check_time,
    //         Some(Instant::from_ticks(1234))
    //     );
    //     assert_eq!(network.status.csd.get_status(), Status::None);
    //     assert_eq!(network.status.csd.updated(), None);
    //     assert_eq!(network.status.csd.started(), None);
    //     assert_eq!(network.status.psd.get_status(), Status::None);
    //     assert_eq!(network.status.psd.updated(), None);
    //     assert_eq!(network.status.psd.started(), None);
    //     assert_eq!(network.status.eps.get_status(), Status::None);
    //     assert_eq!(network.status.eps.updated(), None);
    //     assert_eq!(network.status.eps.started(), None);

    //     // Check that `ConnectionState` will change from `Connecting` to `Connected`
    //     // if eps is actually registered
    //     network.status.eps.set_status(Status::Roaming);

    //     network.check_registration_state();

    //     assert_eq!(network.status.conn_state, ConnectionState::Connected);

    //     // Check that `ConnectionState` will change from `Connecting` to `Connected`
    //     // if (csd + psd) is actually registered
    //     network.status.conn_state = ConnectionState::Connecting;
    //     network.status.reset();
    //     network.status.eps.set_status(Status::Denied);
    //     network.status.csd.set_status(Status::Roaming);
    //     network.status.psd.set_status(Status::Home);

    //     network.check_registration_state();

    //     assert_eq!(network.status.conn_state, ConnectionState::Connected);
    // }

    // #[test]
    // fn unhandled_urcs() {
    //     let mut tx = AtTx::new(MockAtClient::new(0), 5);

    //     tx.handle_urc(|_| false).unwrap();
    //     assert_eq!(tx.client.n_urcs_dequeued, 0);
    //     tx.handle_urc(|_| false).unwrap();
    //     tx.handle_urc(|_| false).unwrap();
    //     tx.handle_urc(|_| false).unwrap();
    //     tx.handle_urc(|_| false).unwrap();
    //     tx.handle_urc(|_| false).unwrap();
    //     assert_eq!(tx.client.n_urcs_dequeued, 1);
    //     tx.handle_urc(|_| false).unwrap();
    //     tx.handle_urc(|_| true).unwrap();
    //     tx.handle_urc(|_| false).unwrap();
    //     assert_eq!(tx.client.n_urcs_dequeued, 2);
    // }
}
