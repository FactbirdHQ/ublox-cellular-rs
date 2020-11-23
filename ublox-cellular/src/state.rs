use crate::{
    command::network_service,
    command::network_service::types::RatAct,
    command::psn::responses::GPRSNetworkRegistrationStatus,
    command::psn::types::GPRSNetworkRegistrationStat,
    command::psn::{responses::EPSNetworkRegistrationStatus, urc::GPRSNetworkRegistration},
    command::psn::{types::EPSNetworkRegistrationStat, urc::EPSNetworkRegistration},
    error::Error,
};
use embedded_hal::timer::CountDown;
use heapless::{consts, String, Vec};
use network_service::types::NetworkRegistrationStat;

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum State {
    Init,
    PowerOn,
    Configure,
    DeviceReady,
    SimPin,
    SignalQuality,
    RegisteringNetwork,
    AttachingNetwork,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct StateMachine {
    max_retry_attempts: u8,
    retry_count: u8,
    inner: State,
}

impl StateMachine {
    pub(crate) const fn new() -> Self {
        Self {
            max_retry_attempts: 10,
            retry_count: 0,
            inner: State::Init,
        }
    }

    pub(crate) fn set_max_retry_attempts(&mut self, max_retry_attempts: u8) {
        self.max_retry_attempts = max_retry_attempts;
    }

    pub(crate) fn reset(&mut self) {
        self.max_retry_attempts = 10;
        self.retry_count = 0;
    }

    pub(crate) const fn get_state(self) -> State {
        self.inner
    }

    pub(crate) fn set_state(&mut self, new_state: State) {
        defmt::debug!("State transition: {:?} -> {:?}", self.inner, new_state);

        // Reset the max attempts on any state transition
        self.reset();
        self.inner = new_state;
    }

    pub(crate) const fn is_retry(self) -> bool {
        self.retry_count > 0
    }

    pub(crate) fn retry_or_fail<CNT>(&mut self, timer: &mut CNT) -> nb::Error<Error>
    where
        CNT: CountDown,
        CNT::Time: From<u32>,
    {
        // Handle retry based on exponential backoff here
        if self.is_retry() && self.retry_count >= self.max_retry_attempts {
            // Max attempts reached! Bail with a timeout error
            return nb::Error::Other(Error::StateTimeout);
        }

        // TODO: Change to a poor-mans exponential
        let backoff_time = (u32::from(self.retry_count) + 1) * 1000;

        if timer.try_start(backoff_time).is_err() {
            defmt::error!("Failed to start retry_timer!!");
            return nb::Error::Other(Error::_Unknown);
        }

        defmt::warn!(
            "[RETRY] current attempt: {:u8}, retrying state({:?}) in {:u32} ms...",
            self.retry_count,
            self.inner,
            backoff_time
        );

        self.retry_count += 1;
        nb::Error::WouldBlock
    }
}

pub struct RegistrationParams {
    reg_type: RadioAccessNetwork,
    status: RegistrationStatus,
    act: RatAct,

    cell_id: Option<String<consts::U8>>,
    lac: Option<String<consts::U4>>,
    // active_time: Option<u16>,
    // periodic_tau: Option<u16>,
}

#[derive(Default)]
pub struct Registration {
    params: RegistrationParams,
    pub events: Vec<Event, consts::U10>,
}

pub enum Event {
    Disconnected,
    CellularRadioAccessTechnologyChanged(RadioAccessNetwork, RatAct),
    CellularRegistrationStatusChanged(RadioAccessNetwork, RegistrationStatus),
    CellularCellIDChanged(Option<String<consts::U8>>),
}

impl Default for RegistrationParams {
    fn default() -> Self {
        Self {
            reg_type: RadioAccessNetwork::UnknownUnused,
            status: RegistrationStatus::StatusNotAvailable,
            act: RatAct::Unknown,
            cell_id: None,
            lac: None,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl Registration {
    pub fn set_params(&mut self, new_params: RegistrationParams) {
        self.params = new_params;
    }

    pub fn compare_and_set(&mut self, new_params: RegistrationParams) {
        if self.params.act != new_params.act {
            self.params.act = new_params.act;
            self.events
                .push(Event::CellularRadioAccessTechnologyChanged(
                    new_params.reg_type,
                    self.params.act,
                ))
                .ok();
        }
        if self.params.status != new_params.status || self.params.reg_type != new_params.reg_type {
            let prev_status = self.params.status;

            self.params.status = new_params.status;
            self.events
                .push(Event::CellularRegistrationStatusChanged(
                    new_params.reg_type,
                    self.params.status,
                ))
                .ok();

            if new_params.status == RegistrationStatus::NotRegistered
                && prev_status.is_registered().is_some()
                && new_params.reg_type != RadioAccessNetwork::Geran
            {
                self.events.push(Event::Disconnected).ok();
            }
        }
        if new_params.cell_id.is_some() && self.params.cell_id != new_params.cell_id {
            self.params.cell_id = new_params.cell_id.clone();
            self.params.lac = new_params.lac;
            self.events
                .push(Event::CellularCellIDChanged(new_params.cell_id))
                .ok();
        }

        self.params.reg_type = new_params.reg_type;
    }

    /// Check if any Radio Access Network is registered
    pub const fn is_registered(&self) -> Option<RegistrationStatus> {
        self.params.status.is_registered()
    }

    /// Check if any Radio Access Network is registered
    pub const fn is_denied(&self) -> bool {
        self.params.status.is_denied()
    }
}

impl From<network_service::urc::NetworkRegistration> for RegistrationParams {
    fn from(v: network_service::urc::NetworkRegistration) -> Self {
        Self {
            reg_type: RadioAccessNetwork::Geran,
            status: v.stat.into(),
            act: RatAct::Gsm,
            cell_id: None,
            lac: None,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<GPRSNetworkRegistration> for RegistrationParams {
    fn from(v: GPRSNetworkRegistration) -> Self {
        Self {
            reg_type: RadioAccessNetwork::Utran,
            status: v.stat.into(),
            act: v.act.unwrap_or(RatAct::Unknown),
            cell_id: v.ci,
            lac: v.lac,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<GPRSNetworkRegistrationStatus> for RegistrationParams {
    fn from(v: GPRSNetworkRegistrationStatus) -> Self {
        Self {
            reg_type: RadioAccessNetwork::Utran,
            status: v.stat.into(),
            act: v.act.unwrap_or(RatAct::Unknown),
            cell_id: v.ci,
            lac: v.lac,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<EPSNetworkRegistration> for RegistrationParams {
    fn from(v: EPSNetworkRegistration) -> Self {
        Self {
            reg_type: RadioAccessNetwork::Eutran,
            status: v.stat.into(),
            act: v.act.unwrap_or(RatAct::Unknown),
            cell_id: v.ci,
            lac: v.tac,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<EPSNetworkRegistrationStatus> for RegistrationParams {
    fn from(v: EPSNetworkRegistrationStatus) -> Self {
        Self {
            reg_type: RadioAccessNetwork::Eutran,
            status: v.stat.into(),
            act: v.act.unwrap_or(RatAct::Unknown),
            cell_id: v.ci,
            lac: v.tac,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum RegistrationStatus {
    /// State is unknown/uninitialized
    StatusNotAvailable,
    /// Not registered
    NotRegistered,
    RegisteredHomeNetwork,
    SearchingNetwork,
    RegistrationDenied,
    Unknown,
    RegisteredRoaming,
    RegisteredSMSOnlyHome,
    RegisteredSMSOnlyRoaming,
    AttachedEmergencyOnly,
    RegisteredCSFBNotPreferredHome,
    RegisteredCSFBNotPreferredRoaming,
    AlreadyRegistered,
}

impl RegistrationStatus {
    pub const fn is_registered(self) -> Option<Self> {
        match self {
            Self::RegisteredHomeNetwork | Self::RegisteredRoaming => Some(self),
            _ => None,
        }
    }

    pub const fn is_roaming(self) -> bool {
        matches!(
            self,
            Self::RegisteredRoaming
                | Self::RegisteredSMSOnlyRoaming
                | Self::RegisteredCSFBNotPreferredRoaming
        )
    }

    pub const fn is_attempting(self) -> bool {
        !matches!(self, Self::NotRegistered | Self::RegistrationDenied)
    }

    pub const fn is_denied(self) -> bool {
        matches!(self, Self::RegistrationDenied)
    }
}

/// Convert the 3GPP registration status from a CREG URC to [`RegistrationStatus`].
impl From<NetworkRegistrationStat> for RegistrationStatus {
    fn from(v: NetworkRegistrationStat) -> Self {
        match v {
            NetworkRegistrationStat::NotRegistered => Self::NotRegistered,
            NetworkRegistrationStat::Registered => Self::RegisteredHomeNetwork,
            NetworkRegistrationStat::NotRegisteredSearching => Self::SearchingNetwork,
            NetworkRegistrationStat::RegistrationDenied => Self::RegistrationDenied,
            NetworkRegistrationStat::Unknown => Self::Unknown,
            NetworkRegistrationStat::RegisteredRoaming => Self::RegisteredRoaming,
            NetworkRegistrationStat::RegisteredSmsOnly => Self::RegisteredSMSOnlyHome,
            NetworkRegistrationStat::RegisteredSmsOnlyRoaming => Self::RegisteredSMSOnlyRoaming,
            NetworkRegistrationStat::RegisteredCsfbNotPerferred => {
                Self::RegisteredCSFBNotPreferredHome
            }
            NetworkRegistrationStat::RegisteredCsfbNotPerferredRoaming => {
                Self::RegisteredCSFBNotPreferredRoaming
            }
        }
    }
}

/// Convert the 3GPP registration status from a CGREG URC to [`RegistrationStatus`].
impl From<GPRSNetworkRegistrationStat> for RegistrationStatus {
    fn from(v: GPRSNetworkRegistrationStat) -> Self {
        match v {
            GPRSNetworkRegistrationStat::NotRegistered => Self::NotRegistered,
            GPRSNetworkRegistrationStat::Registered => Self::RegisteredHomeNetwork,
            GPRSNetworkRegistrationStat::NotRegisteredSearching => Self::SearchingNetwork,
            GPRSNetworkRegistrationStat::RegistrationDenied => Self::RegistrationDenied,
            GPRSNetworkRegistrationStat::Unknown => Self::Unknown,
            GPRSNetworkRegistrationStat::RegisteredRoaming => Self::RegisteredRoaming,
            GPRSNetworkRegistrationStat::AttachedEmergencyOnly => Self::AttachedEmergencyOnly,
        }
    }
}

/// Convert the 3GPP registration status from a CEREG URC to [`RegistrationStatus`].
impl From<EPSNetworkRegistrationStat> for RegistrationStatus {
    fn from(v: EPSNetworkRegistrationStat) -> Self {
        match v {
            EPSNetworkRegistrationStat::NotRegistered => Self::NotRegistered,
            EPSNetworkRegistrationStat::Registered => Self::RegisteredHomeNetwork,
            EPSNetworkRegistrationStat::NotRegisteredSearching => Self::SearchingNetwork,
            EPSNetworkRegistrationStat::RegistrationDenied => Self::RegistrationDenied,
            EPSNetworkRegistrationStat::Unknown => Self::Unknown,
            EPSNetworkRegistrationStat::RegisteredRoaming => Self::RegisteredRoaming,
            EPSNetworkRegistrationStat::AttachedEmergencyOnly => Self::AttachedEmergencyOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum RadioAccessNetwork {
    UnknownUnused = 0,
    Geran = 1,
    Utran = 2,
    Eutran = 3,
}

impl From<usize> for RadioAccessNetwork {
    fn from(v: usize) -> Self {
        match v {
            1 => Self::Geran,
            2 => Self::Utran,
            3 => Self::Eutran,
            _ => Self::UnknownUnused,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct MockTimer {
        time: Option<u32>,
    }

    impl MockTimer {
        pub fn new() -> Self {
            MockTimer { time: None }
        }
    }

    impl CountDown for MockTimer {
        type Error = core::convert::Infallible;
        type Time = u32;
        fn try_start<T>(&mut self, count: T) -> Result<(), Self::Error>
        where
            T: Into<Self::Time>,
        {
            self.time = Some(count.into());
            Ok(())
        }
        fn try_wait(&mut self) -> nb::Result<(), Self::Error> {
            self.time = None;
            Ok(())
        }
    }

    #[test]
    fn retry_or_fail() {
        let mut fsm = StateMachine::new();
        assert_eq!(fsm.get_state(), State::Init);
        assert!(!fsm.is_retry());

        let mut timer = MockTimer::new();

        let max_attempts = fsm.max_retry_attempts;

        for i in 0..(max_attempts + 1) {
            match fsm.retry_or_fail(&mut timer) {
                nb::Error::WouldBlock if i >= max_attempts => panic!(),
                nb::Error::Other(Error::StateTimeout) if i < max_attempts => panic!(),
                nb::Error::Other(Error::StateTimeout) => {}
                nb::Error::Other(e) => panic!("Got unexpected error {:?}", e),
                _ => {}
            }
            assert!(fsm.is_retry());

            if i < max_attempts {
                assert_eq!(timer.time, Some((i as u32 + 1) * 1000));
                timer.try_wait().unwrap();
            } else {
                assert!(timer.time.is_none());
            }
        }
    }
}
