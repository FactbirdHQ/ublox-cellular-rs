use super::Clock;
use crate::{
    command::{
        error::UbloxError,
        mobile_control::{
            types::{Functionality, ResetMode},
            SetModuleFunctionality,
        },
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
    registration::{self, ConnectionState, RegistrationParams, RegistrationState},
    services::data::ContextState,
};
use atat::{atat_derive::AtatLen, AtatClient};
use fugit::{ExtU32, MinutesDurationU32, SecsDurationU32};
use hash32_derive::Hash32;
use serde::{Deserialize, Serialize};

const REGISTRATION_CHECK_INTERVAL: SecsDurationU32 = SecsDurationU32::secs(15);
const REGISTRATION_TIMEOUT: MinutesDurationU32 = MinutesDurationU32::minutes(5);

#[derive(Debug, PartialEq)]
pub enum Error {
    Generic(GenericError),
    AT(atat::Error<UbloxError>),
    RegistrationDenied,
    UnknownProfile,
    ActivationFailed,
    _Unknown,
}

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Hash32, Serialize, Deserialize, AtatLen, defmt::Format,
)]
pub struct ProfileId(pub u8);

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, AtatLen, defmt::Format)]
pub struct ContextId(pub u8);

pub struct AtTx<C> {
    urc_attempts: u8,
    max_urc_attempts: u8,
    consecutive_timeouts: u8,
    client: C,
}

impl<C: AtatClient> AtTx<C> {
    pub fn new(client: C, max_urc_attempts: u8) -> Self {
        Self {
            urc_attempts: 0,
            consecutive_timeouts: 0,
            max_urc_attempts,
            client,
        }
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.client.reset();
        Ok(())
    }

    pub fn send_ignore_timeout<A, const LEN: usize>(
        &mut self,
        req: &A,
    ) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
        A::Error: Into<UbloxError>,
    {
        self.client
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    // let request = req.as_bytes();

                    if !matches!(ate, atat::Error::Timeout) {
                        // defmt::error!("{}: [{=[u8]:a}]", ate, request[..request.len() - 2]);
                    }

                    match ate {
                        atat::Error::Error(ubx) => {
                            let u: UbloxError = ubx.into();
                            Error::AT(atat::Error::Error(u))
                        }
                        atat::Error::Timeout => {
                            self.consecutive_timeouts += 1;
                            Error::AT(atat::Error::Timeout)
                        }
                        atat::Error::Read => Error::AT(atat::Error::Read),
                        atat::Error::Write => Error::AT(atat::Error::Write),
                        atat::Error::InvalidResponse => Error::AT(atat::Error::InvalidResponse),
                        atat::Error::Aborted => Error::AT(atat::Error::Aborted),
                        atat::Error::Overflow => Error::AT(atat::Error::Overflow),
                        atat::Error::Parse => Error::AT(atat::Error::Parse),
                    }
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
            .map(|res| {
                self.consecutive_timeouts = 0;
                res
            })
    }

    pub fn send<A, const LEN: usize>(&mut self, req: &A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
        A::Error: Into<UbloxError>,
    {
        self.client
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    // let request = req.as_bytes();
                    // defmt::error!("{}: [{=[u8]:a}]", ate, request[..request.len() - 2]);

                    match ate {
                        atat::Error::Error(ubx) => {
                            let u: UbloxError = ubx.into();
                            Error::AT(atat::Error::Error(u))
                        }
                        atat::Error::Timeout => {
                            self.consecutive_timeouts += 1;
                            Error::AT(atat::Error::Timeout)
                        }
                        atat::Error::Read => Error::AT(atat::Error::Read),
                        atat::Error::Write => Error::AT(atat::Error::Write),
                        atat::Error::InvalidResponse => Error::AT(atat::Error::InvalidResponse),
                        atat::Error::Aborted => Error::AT(atat::Error::Aborted),
                        atat::Error::Overflow => Error::AT(atat::Error::Overflow),
                        atat::Error::Parse => Error::AT(atat::Error::Parse),
                    }
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
            .map(|res| {
                self.consecutive_timeouts = 0;
                res
            })
    }

    pub fn handle_urc<F: FnOnce(Urc) -> bool>(&mut self, f: F) -> Result<(), Error> {
        let mut a = self.urc_attempts;
        let max = self.max_urc_attempts;

        self.client.peek_urc_with::<Urc, _>(|urc| {
            if !f(urc.clone()) {
                if a < max {
                    a += 1;
                    return false;
                    // } else {
                    // defmt::warn!("Dropping stale URC! {}", defmt::Debug2Format(&urc));
                }
            }
            a = 0;
            true
        });
        self.urc_attempts = a;
        Ok(())
    }
}

pub struct Network<C, CLK, const FREQ_HZ: u32>
where
    CLK: Clock<FREQ_HZ>,
{
    pub(crate) status: RegistrationState<CLK, FREQ_HZ>,
    pub(crate) context_state: ContextState,
    pub(crate) at_tx: AtTx<C>,
}

impl<C, CLK, const FREQ_HZ: u32> Network<C, CLK, FREQ_HZ>
where
    C: AtatClient,
    CLK: Clock<FREQ_HZ>,
{
    pub(crate) fn new(at_tx: AtTx<C>, timer: CLK) -> Self {
        Network {
            status: RegistrationState::new(timer),
            context_state: ContextState::Setup,
            at_tx,
        }
    }

    pub fn is_connected(&self) -> Result<bool, Error> {
        Ok(matches!(self.status.conn_state, ConnectionState::Connected))
    }

    pub fn reset_reg_time(&mut self) -> Result<(), Error> {
        let now = self.status.timer.now();

        self.status.reg_start_time.replace(now);
        self.status.reg_check_time = self.status.reg_start_time;
        Ok(())
    }

    pub fn process_events(&mut self) -> Result<(), Error> {
        if self.at_tx.consecutive_timeouts > 10 {
            defmt::warn!("Resetting the modem due to consecutive AT timeouts");
            return Err(Error::Generic(GenericError::Timeout));
        }

        self.handle_urc()?;
        self.check_registration_state()?;
        self.intervene_registration()?;
        // self.check_running_imsi();

        let now = self.status.timer.now();
        let should_check = self
            .status
            .reg_check_time
            .and_then(|reg_check_time| {
                now.checked_duration_since(reg_check_time)
                    .map(|dur| dur >= REGISTRATION_CHECK_INTERVAL)
            })
            .unwrap_or(true);

        if self.status.conn_state != ConnectionState::Connecting || !should_check {
            return Ok(());
        }

        self.status.reg_check_time.replace(now);

        self.update_registration()?;

        let now = self.status.timer.now();
        let is_timeout = self
            .status
            .reg_start_time
            .and_then(|reg_start_time| {
                now.checked_duration_since(reg_start_time)
                    .map(|dur| dur >= REGISTRATION_TIMEOUT)
            })
            .unwrap_or(false);

        if self.status.conn_state == ConnectionState::Connecting && is_timeout {
            defmt::warn!("Resetting the modem due to the network registration timeout");

            return Err(Error::Generic(GenericError::Timeout));
        }
        Ok(())
    }

    pub fn check_registration_state(&mut self) -> Result<(), Error> {
        // Don't do anything if we are actually disconnected by choice
        if self.status.conn_state == ConnectionState::Disconnected {
            return Ok(());
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

        Ok(())
    }

    pub fn intervene_registration(&mut self) -> Result<(), Error> {
        if self.status.conn_state != ConnectionState::Connecting {
            return Ok(());
        }

        let now = self.status.timer.now();

        // If EPS has been sticky for longer than `timeout`
        let timeout: SecsDurationU32 = (self.status.registration_interventions * 15).secs();
        if self.status.eps.sticky() && self.status.eps.duration(now) >= timeout {
            // If (EPS + CSD) is not attempting registration
            if self.status.eps.get_status() == registration::Status::NotRegistering
                && self.status.csd.get_status() == registration::Status::NotRegistering
            {
                defmt::debug!(
                    "Sticky not registering state for {}, PLMN reselection",
                    self.status.eps.duration(now)
                );

                self.status.csd.reset();
                self.status.psd.reset();
                self.status.eps.reset();
                self.status.registration_interventions += 1;
                self.send_internal(
                    &SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                        format: Some(2),
                    },
                    false,
                )
                .ok();
                return Ok(());

            // If (EPS + CSD) is denied registration
            } else if self.status.eps.get_status() == registration::Status::Denied
                && self.status.csd.get_status() == registration::Status::Denied
            {
                defmt::debug!(
                    "Sticky denied state for {}, RF reset",
                    self.status.eps.duration(now)
                );
                self.status.csd.reset();
                self.status.psd.reset();
                self.status.eps.reset();
                self.status.registration_interventions += 1;
                self.send_internal(
                    &SetModuleFunctionality {
                        fun: Functionality::Minimum,
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
            && self.status.csd.get_status() == registration::Status::Denied
            && self.status.psd.get_status() == registration::Status::Denied
        {
            defmt::debug!(
                "Sticky CSD and PSD denied state for {}, RF reset",
                self.status.csd.duration(now)
            );
            self.status.csd.reset();
            self.status.psd.reset();
            self.status.eps.reset();
            self.status.registration_interventions += 1;
            self.send_internal(
                &SetModuleFunctionality {
                    fun: Functionality::Minimum,
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
            defmt::debug!(
                "Sticky not registering PSD state for {}, force GPRS attach",
                self.status.psd.duration(now)
            );
            self.status.psd.reset();
            self.status.registration_interventions += 1;
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
                defmt::warn!("GPRS attach failed, try PLMN reselection");
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
        let ts = self.status.timer.now();

        if let Ok(reg) = self.send_internal(&GetNetworkRegistrationStatus, false) {
            self.status.compare_and_set(reg.into(), ts);
        }

        if let Ok(reg) = self.send_internal(&GetGPRSNetworkRegistrationStatus, false) {
            self.status.compare_and_set(reg.into(), ts);
        }

        if let Ok(reg) = self.send_internal(&GetEPSNetworkRegistrationStatus, false) {
            self.status.compare_and_set(reg.into(), ts);
        }

        Ok(())
    }

    pub(crate) fn handle_urc(&mut self) -> Result<(), Error> {
        // TODO: How to do this cleaner?
        let mut ctx_state = self.context_state;
        let mut new_reg_params: Option<RegistrationParams> = None;

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
                    defmt::info!("[URC] ExtendedPSNetworkRegistration {}", state);
                }
                Urc::GPRSNetworkRegistration(reg_params) => {
                    new_reg_params.replace(reg_params.into());
                }
                Urc::EPSNetworkRegistration(reg_params) => {
                    new_reg_params.replace(reg_params.into());
                }
                Urc::NetworkRegistration(reg_params) => {
                    new_reg_params.replace(reg_params.into());
                }
                Urc::DataConnectionActivated(psn::urc::DataConnectionActivated {
                    result,
                    ip_addr: _,
                }) => {
                    defmt::info!("[URC] DataConnectionActivated {=u8}", result);
                    ctx_state = ContextState::Active;
                }
                Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    profile_id,
                }) => {
                    defmt::info!("[URC] DataConnectionDeactivated {}", profile_id);
                    ctx_state = ContextState::Activating;
                }
                Urc::MessageWaitingIndication(_) => {
                    defmt::info!("[URC] MessageWaitingIndication");
                }
                _ => return false,
            };
            true
        })?;

        if let Some(reg_params) = new_reg_params {
            let ts = self.status.timer.now();
            self.status.compare_and_set(reg_params, ts)
        }

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
        A::Error: Into<UbloxError>,
    {
        if check_urc {
            if let Err(e) = self.handle_urc() {
                defmt::error!("Failed handle URC  {}", defmt::Debug2Format(&e));
            }
        }

        self.at_tx.send(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        registration::Status,
        test_helpers::{MockAtClient, MockTimer},
        Instant,
    };
    use fugit::MillisDurationU32;

    const FREQ_HZ: u32 = 1000;

    #[test]
    #[ignore]
    fn intervene_registration() {
        // Setup
        let tx = AtTx::new(MockAtClient::new(0), 5);
        let timer: MockTimer<FREQ_HZ> = MockTimer::new(Some(Instant::from_ticks(25_234)));
        let mut network = Network::new(tx, timer);
        network.status.conn_state = ConnectionState::Connecting;
        // Update both started & updated
        network
            .status
            .eps
            .set_status(Status::NotRegistering, Instant::from_ticks(1234));
        // Update only updated
        network
            .status
            .eps
            .set_status(Status::NotRegistering, Instant::from_ticks(1534));
        network
            .status
            .csd
            .set_status(Status::NotRegistering, Instant::from_ticks(1534));

        assert_eq!(
            network.status.eps.updated(),
            Some(Instant::from_ticks(1534))
        );
        assert_eq!(
            network.status.eps.started(),
            Some(Instant::from_ticks(1234))
        );
        assert!(network.status.eps.sticky());

        let ts = network.status.timer.now();
        assert_eq!(
            network.status.eps.duration(ts),
            MillisDurationU32::millis(24_000)
        );

        assert!(network.intervene_registration().is_ok());

        assert_eq!(network.status.registration_interventions, 2);
    }

    #[test]
    fn reset_reg_time() {
        let tx = AtTx::new(MockAtClient::new(0), 5);
        let timer: MockTimer<FREQ_HZ> = MockTimer::new(Some(Instant::from_ticks(1234)));
        let mut network = Network::new(tx, timer);

        assert!(network.reset_reg_time().is_ok());

        assert_eq!(
            network.status.reg_start_time,
            Some(Instant::from_ticks(1234))
        );
        assert_eq!(
            network.status.reg_check_time,
            Some(Instant::from_ticks(1234))
        );
    }

    #[test]
    fn check_registration_state() {
        let tx = AtTx::new(MockAtClient::new(0), 5);
        let timer: MockTimer<FREQ_HZ> = MockTimer::new(Some(Instant::from_ticks(1234)));
        let mut network = Network::new(tx, timer);

        // Check that `ConnectionState` will change from `Connected` to `Connecting`
        // with a state reset, if neither (csd + psd) || eps is actually registered
        network.status.conn_state = ConnectionState::Connected;
        network.status.registration_interventions = 3;
        network
            .status
            .csd
            .set_status(Status::Denied, Instant::from_ticks(1));
        network
            .status
            .eps
            .set_status(Status::NotRegistering, Instant::from_ticks(5));

        assert!(network.check_registration_state().is_ok());

        assert_eq!(network.status.conn_state, ConnectionState::Connecting);
        assert_eq!(
            network.status.reg_start_time,
            Some(Instant::from_ticks(1234))
        );
        assert_eq!(
            network.status.reg_check_time,
            Some(Instant::from_ticks(1234))
        );
        assert_eq!(network.status.csd.get_status(), Status::None);
        assert_eq!(network.status.csd.updated(), None);
        assert_eq!(network.status.csd.started(), None);
        assert_eq!(network.status.psd.get_status(), Status::None);
        assert_eq!(network.status.psd.updated(), None);
        assert_eq!(network.status.psd.started(), None);
        assert_eq!(network.status.eps.get_status(), Status::None);
        assert_eq!(network.status.eps.updated(), None);
        assert_eq!(network.status.eps.started(), None);

        // Check that `ConnectionState` will change from `Connecting` to `Connected`
        // if eps is actually registered
        network
            .status
            .eps
            .set_status(Status::Roaming, Instant::from_ticks(5));

        assert!(network.check_registration_state().is_ok());

        assert_eq!(network.status.conn_state, ConnectionState::Connected);

        // Check that `ConnectionState` will change from `Connecting` to `Connected`
        // if (csd + psd) is actually registered
        network.status.conn_state = ConnectionState::Connecting;
        network.status.reset();
        network
            .status
            .eps
            .set_status(Status::Denied, Instant::from_ticks(5));
        network
            .status
            .csd
            .set_status(Status::Roaming, Instant::from_ticks(5));
        network
            .status
            .psd
            .set_status(Status::Home, Instant::from_ticks(5));

        assert!(network.check_registration_state().is_ok());

        assert_eq!(network.status.conn_state, ConnectionState::Connected);
    }

    #[test]
    fn unhandled_urcs() {
        let mut tx = AtTx::new(MockAtClient::new(0), 5);

        tx.handle_urc(|_| false).unwrap();
        assert_eq!(tx.client.n_urcs_dequeued, 0);
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| false).unwrap();
        assert_eq!(tx.client.n_urcs_dequeued, 1);
        tx.handle_urc(|_| false).unwrap();
        tx.handle_urc(|_| true).unwrap();
        tx.handle_urc(|_| false).unwrap();
        assert_eq!(tx.client.n_urcs_dequeued, 2);
    }
}
