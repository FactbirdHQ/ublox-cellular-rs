use crate::{command::*, error::Error};
use embedded_hal::timer::CountDown;
use network_service::types::NetworkRegistrationStat;
use psn::types::{EPSNetworkRegistrationStat, GPRSNetworkRegistrationStat};

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
    pub(crate) fn new() -> Self {
        StateMachine {
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

    pub(crate) fn get_state(&self) -> State {
        self.inner
    }

    pub(crate) fn set_state(&mut self, new_state: State) {
        defmt::debug!("State transition: {:?} -> {:?}", self.inner, new_state);

        // Reset the max attempts on any state transition
        self.reset();
        self.inner = new_state;
    }

    pub(crate) fn is_retry(&self) -> bool {
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
        let backoff_time = (self.retry_count as u32 + 1) * 1000;

        if timer.try_start(backoff_time).is_err() {
            return nb::Error::Other(Error::_Unknown);
        }

        defmt::warn!(
            "[RETRY] current attempt: {:u8}, retrying state in {:u32} ms...",
            self.retry_count,
            backoff_time
        );

        self.retry_count += 1;
        nb::Error::WouldBlock
    }
}

pub struct RANStatus([RegistrationStatus; 4]);

impl RANStatus {
    pub fn new() -> Self {
        Self([RegistrationStatus::StatusNotAvailable; 4])
    }

    /// Set the network status of a given Radio Access Network
    pub fn set(&mut self, ran: RadioAccessNetwork, status: RegistrationStatus) {
        if let Some(s) = self.0.get_mut(ran as usize) {
            defmt::debug!("Setting {:?} to {:?}", ran, status);
            *s = status;
        }
    }

    /// Get the network status of a given Radio Access Network
    pub fn get(&self, ran: RadioAccessNetwork) -> RegistrationStatus {
        *self
            .0
            .get(ran as usize)
            .unwrap_or(&RegistrationStatus::Unknown)
    }

    /// Check if any Radio Access Network is registered
    pub fn is_registered(&self) -> Option<RegistrationStatus> {
        if let Some(utran) = self.get(RadioAccessNetwork::Utran).is_registered() {
            return Some(utran);
        }
        if let Some(eutran) = self.get(RadioAccessNetwork::Eutran).is_registered() {
            return Some(eutran);
        }

        None
    }

    /// Check if we are currently roaming on any Radio Access Network
    pub fn is_roaming(&self) -> bool {
        self.get(RadioAccessNetwork::Utran).is_roaming()
            || self.get(RadioAccessNetwork::Eutran).is_roaming()
    }

    /// Check if we are currently denied registration on any Radio Access Network
    pub fn is_denied(&self) -> bool {
        self.get(RadioAccessNetwork::Utran) == RegistrationStatus::RegistrationDenied
            || self.get(RadioAccessNetwork::Eutran) == RegistrationStatus::RegistrationDenied
        // || self.get(RadioAccessNetwork::Utran) == RegistrationStatus::NotRegistered
        // || self.get(RadioAccessNetwork::Eutran) == RegistrationStatus::NotRegistered
    }

    pub fn is_attempting(&self) -> bool {
        self.get(RadioAccessNetwork::Utran).is_attempting()
            || self.get(RadioAccessNetwork::Eutran).is_attempting()
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
    pub fn is_registered(&self) -> Option<RegistrationStatus> {
        use RegistrationStatus::*;

        match self {
            RegisteredHomeNetwork | RegisteredRoaming => Some(*self),
            _ => None,
        }
    }

    pub fn is_roaming(&self) -> bool {
        use RegistrationStatus::*;

        matches!(
            self,
            RegisteredRoaming | RegisteredSMSOnlyRoaming | RegisteredCSFBNotPreferredRoaming
        )
    }

    pub fn is_attempting(&self) -> bool {
        use RegistrationStatus::*;

        !matches!(self, NotRegistered | RegistrationDenied)
    }
}

/// Convert the 3GPP registration status from a CREG URC to RegistrationStatus.
impl From<NetworkRegistrationStat> for RegistrationStatus {
    fn from(v: NetworkRegistrationStat) -> Self {
        use NetworkRegistrationStat::*;

        match v {
            NotRegistered => RegistrationStatus::NotRegistered,
            Registered => RegistrationStatus::RegisteredHomeNetwork,
            NotRegisteredSearching => RegistrationStatus::SearchingNetwork,
            RegistrationDenied => RegistrationStatus::RegistrationDenied,
            Unknown => RegistrationStatus::Unknown,
            RegisteredRoaming => RegistrationStatus::RegisteredRoaming,
            RegisteredSmsOnly => RegistrationStatus::RegisteredSMSOnlyHome,
            RegisteredSmsOnlyRoaming => RegistrationStatus::RegisteredSMSOnlyRoaming,
            RegisteredCsfbNotPerferred => RegistrationStatus::RegisteredCSFBNotPreferredHome,
            RegisteredCsfbNotPerferredRoaming => {
                RegistrationStatus::RegisteredCSFBNotPreferredRoaming
            }
        }
    }
}

/// Convert the 3GPP registration status from a CGREG URC to RegistrationStatus.
impl From<GPRSNetworkRegistrationStat> for RegistrationStatus {
    fn from(v: GPRSNetworkRegistrationStat) -> Self {
        use GPRSNetworkRegistrationStat::*;

        match v {
            NotRegistered => RegistrationStatus::NotRegistered,
            Registered => RegistrationStatus::RegisteredHomeNetwork,
            NotRegisteredSearching => RegistrationStatus::SearchingNetwork,
            RegistrationDenied => RegistrationStatus::RegistrationDenied,
            Unknown => RegistrationStatus::Unknown,
            RegisteredRoaming => RegistrationStatus::RegisteredRoaming,
            AttachedEmergencyOnly => RegistrationStatus::AttachedEmergencyOnly,
        }
    }
}

/// Convert the 3GPP registration status from a CEREG URC to RegistrationStatus.
impl From<EPSNetworkRegistrationStat> for RegistrationStatus {
    fn from(v: EPSNetworkRegistrationStat) -> Self {
        use EPSNetworkRegistrationStat::*;

        match v {
            NotRegistered => RegistrationStatus::NotRegistered,
            Registered => RegistrationStatus::RegisteredHomeNetwork,
            NotRegisteredSearching => RegistrationStatus::SearchingNetwork,
            RegistrationDenied => RegistrationStatus::RegistrationDenied,
            Unknown => RegistrationStatus::Unknown,
            RegisteredRoaming => RegistrationStatus::RegisteredRoaming,
            AttachedEmergencyOnly => RegistrationStatus::AttachedEmergencyOnly,
        }
    }
}

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum RadioAccessNetwork {
    UnknownUnused = 0,
    Geran = 1,
    Utran = 2,
    Eutran = 3,
}

impl From<usize> for RadioAccessNetwork {
    fn from(v: usize) -> Self {
        match v {
            1 => RadioAccessNetwork::Geran,
            2 => RadioAccessNetwork::Utran,
            3 => RadioAccessNetwork::Eutran,
            _ => RadioAccessNetwork::UnknownUnused,
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
