use crate::{command::*, error::Error};
use embedded_hal::timer::CountDown;
use network_service::types::NetworkRegistrationStat;
use psn::types::{EPSNetworkRegistrationStat, GPRSNetworkRegistrationStat};

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum State {
    Init,
    PowerOn,
    Configure,
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
        if self.is_retry() {
            if self.retry_count >= self.max_retry_attempts {
                // Max attempts reached! Bail with a timeout error
                return nb::Error::Other(Error::StateTimeout);
            }
        }
        let backoff_time = (self.retry_count as u32 + 1) * 1000;

        if let Err(_) = timer.try_start(backoff_time) {
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

pub struct RANStatus([NetworkStatus; 4]);

impl RANStatus {
    pub fn new() -> Self {
        Self([NetworkStatus::Unknown; 4])
    }

    /// Set the network status of a given Radio Access Network
    pub fn set(&mut self, ran: RadioAccessNetwork, status: NetworkStatus) {
        if let Some(s) = self.0.get_mut(ran as usize) {
            defmt::debug!("Setting {:?} to {:?}", ran, status);
            *s = status;
        }
    }

    /// Get the network status of a given Radio Access Network
    pub fn get(&self, ran: RadioAccessNetwork) -> NetworkStatus {
        *self.0.get(ran as usize).unwrap_or(&NetworkStatus::Unknown)
    }

    /// Check if any Radio Access Network is registered
    pub fn is_registered(&self) -> bool {
        self.get(RadioAccessNetwork::Utran) == NetworkStatus::Registered
            || self.get(RadioAccessNetwork::Eutran) == NetworkStatus::Registered
    }
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum NetworkStatus {
    /// State is unknown/uninitialized
    Unknown,
    /// Searching
    Searching,
    /// Registered on the home network
    Registered,
    /// Registration denied
    RegistrationDenied,
    /// Out of coverage
    OutOfCoverage,
    /// Registered on a roaming network
    NotRegistered,
    /// Emergency service only
    EmergencyOnly,
}

/// Convert the 3GPP registration status from a CREG URC to NetworkStatus.
impl From<NetworkRegistrationStat> for NetworkStatus {
    fn from(v: NetworkRegistrationStat) -> Self {
        match v {
            NetworkRegistrationStat::NotRegistered => NetworkStatus::NotRegistered,
            NetworkRegistrationStat::Registered => NetworkStatus::Registered,
            NetworkRegistrationStat::NotRegisteredSearching => NetworkStatus::Searching,
            NetworkRegistrationStat::RegistrationDenied => NetworkStatus::RegistrationDenied,
            NetworkRegistrationStat::Unknown => NetworkStatus::OutOfCoverage,
            NetworkRegistrationStat::RegisteredRoaming => NetworkStatus::Registered,
            NetworkRegistrationStat::RegisteredSmsOnly => NetworkStatus::NotRegistered,
            NetworkRegistrationStat::RegisteredSmsOnlyRoaming => NetworkStatus::NotRegistered,
            NetworkRegistrationStat::RegisteredCsfbNotPerferred => NetworkStatus::Registered,
            NetworkRegistrationStat::RegisteredCsfbNotPerferredRoaming => NetworkStatus::Registered,
        }
    }
}

/// Convert the 3GPP registration status from a CGREG URC to NetworkStatus.
impl From<GPRSNetworkRegistrationStat> for NetworkStatus {
    fn from(v: GPRSNetworkRegistrationStat) -> Self {
        match v {
            GPRSNetworkRegistrationStat::NotRegistered => NetworkStatus::NotRegistered,
            GPRSNetworkRegistrationStat::Registered => NetworkStatus::Registered,
            GPRSNetworkRegistrationStat::NotRegisteredSearching => NetworkStatus::Searching,
            GPRSNetworkRegistrationStat::RegistrationDenied => NetworkStatus::RegistrationDenied,
            GPRSNetworkRegistrationStat::Unknown => NetworkStatus::OutOfCoverage,
            GPRSNetworkRegistrationStat::RegisteredRoaming => NetworkStatus::Registered,
            GPRSNetworkRegistrationStat::AttachedEmergencyOnly => NetworkStatus::EmergencyOnly,
        }
    }
}

/// Convert the 3GPP registration status from a CEREG URC to NetworkStatus.
impl From<EPSNetworkRegistrationStat> for NetworkStatus {
    fn from(v: EPSNetworkRegistrationStat) -> Self {
        match v {
            EPSNetworkRegistrationStat::NotRegistered => NetworkStatus::NotRegistered,
            EPSNetworkRegistrationStat::Registered => NetworkStatus::Registered,
            EPSNetworkRegistrationStat::NotRegisteredSearching => NetworkStatus::Searching,
            EPSNetworkRegistrationStat::RegistrationDenied => NetworkStatus::RegistrationDenied,
            EPSNetworkRegistrationStat::Unknown => NetworkStatus::OutOfCoverage,
            EPSNetworkRegistrationStat::RegisteredRoaming => NetworkStatus::Registered,
            EPSNetworkRegistrationStat::AttachedEmergencyOnly => NetworkStatus::EmergencyOnly,
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
