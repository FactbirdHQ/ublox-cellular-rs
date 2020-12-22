use core::convert::TryFrom;

use crate::{
    command::network_service,
    command::network_service::responses::NetworkRegistrationStatus,
    command::network_service::types::OperatorNameFormat,
    command::network_service::types::OperatorSelectionMode,
    command::network_service::types::RatAct,
    command::network_service::urc::NetworkRegistration,
    command::psn::responses::GPRSNetworkRegistrationStatus,
    command::psn::types::GPRSNetworkRegistrationStat,
    command::psn::{responses::EPSNetworkRegistrationStatus, urc::GPRSNetworkRegistration},
    command::psn::{types::EPSNetworkRegistrationStat, urc::EPSNetworkRegistration},
    error::Error,
};
use embedded_hal::timer::CountDown;
use heapless::{consts, spsc::Queue, String};
use network_service::types::NetworkRegistrationStat;

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum State {
    /// StateMachine: CELLULAR module is off
    Off,
    /// StateMachine: CELLULAR module is on
    On,
    /// StateMachine: CELLULAR module is connected
    Connected,
    /// StateMachine: CELLULAR module is registered
    Registered,
    /// StateMachine: CELLULAR module is rfoff
    Rfoff,
    /// StateMachine: CELLULAR module is OTA mode
    Ota,
    /// StateMachine: Unknown state
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct StateMachine {
    max_retry_attempts: u8,
    retry_count: Option<u8>,
    inner: State,
}

#[derive(defmt::Format)]
pub enum CellularEvent {
    /// trigger: an module is powered on
    PwrOn,
    /// trigger: cellular module is powered off
    PwrOff,
    /// trigger: airplane mode off
    RfOn,
    /// trigger: airplane mode on
    RfOff,
    /// trigger: attached to a network
    Attached,
    /// trigger: detached from a network
    Detached,
    /// trigger: data connect is active
    DataActive,
    /// trigger: data connection is inactive
    DataInactive,
    /// trigger: OTA starts
    Ota,
    /// trigger: OTA finishes
    OtaDone,
}

impl TryFrom<Event> for CellularEvent {
    type Error = ();

    fn try_from(e: Event) -> Result<Self, Self::Error> {
        Ok(match e {
            Event::RegistrationStatusChanged(reg_type, status) => match reg_type {
                RegType::Cgreg | RegType::Cereg if status.ps_reg_status.is_registered() => {
                    Self::Attached
                }
                RegType::Cgreg | RegType::Cereg if !status.ps_reg_status.is_registered() => {
                    Self::Detached
                }
                RegType::Creg if status.ps_reg_status.is_registered() => {
                    /* CS attach won't count as CELLULAR_EVENT_ATTACHED. */
                    return Err(());
                }
                RegType::Creg if !status.ps_reg_status.is_registered() => Self::Detached,
                _ => {
                    return Err(());
                }
            },
            Event::RadioAccessTechnologyChanged(reg_type, rat) => {
                defmt::info!(
                    "[EVENT] CellularRadioAccessTechnologyChanged {:?} {:?}",
                    reg_type,
                    rat
                );
                return Err(());
            }
            Event::CellIDChanged(cell_id) => {
                defmt::info!(
                    "[EVENT] CellularCellIDChanged {:str}",
                    cell_id.unwrap_or_default().as_str()
                );
                return Err(());
            }
            Event::PwrOn => Self::PwrOn,
            Event::PwrOff => Self::PwrOff,
            Event::RfOn => Self::RfOn,
            Event::RfOff => Self::RfOff,
            Event::Attached => Self::Attached,
            Event::Detached => Self::Detached,
            Event::DataActive => Self::DataActive,
            Event::DataInactive => Self::DataInactive,
            Event::Ota => Self::Ota,
            Event::OtaDone => Self::OtaDone,
        })
    }
}

impl StateMachine {
    pub(crate) const fn new() -> Self {
        Self {
            max_retry_attempts: 10,
            retry_count: None,
            inner: State::Off,
        }
    }

    pub(crate) fn handle_event(&mut self, event: CellularEvent) {
        defmt::debug!("Handling cellular event: {:?}, from {:?}", event, self.get_state());

        let new_state = match self.get_state() {
            State::Off if matches!(event, CellularEvent::PwrOn) => State::On,
            State::Off if matches!(event, CellularEvent::Attached) => State::Registered,
            State::On if matches!(event, CellularEvent::PwrOff) => State::Off,
            State::On if matches!(event, CellularEvent::RfOff) => State::Rfoff,
            State::On if matches!(event, CellularEvent::Attached) => State::Registered,
            State::On if matches!(event, CellularEvent::Detached) => State::On,
            State::Registered if matches!(event, CellularEvent::PwrOff) => State::Off,
            State::Registered if matches!(event, CellularEvent::RfOff) => State::Rfoff,
            State::Registered if matches!(event, CellularEvent::Detached) => State::On,
            State::Registered if matches!(event, CellularEvent::Attached) => State::Registered,
            State::Registered if matches!(event, CellularEvent::DataActive) => State::Connected,
            State::Connected if matches!(event, CellularEvent::PwrOff) => State::Off,
            State::Connected if matches!(event, CellularEvent::RfOff) => State::Rfoff,
            State::Connected if matches!(event, CellularEvent::Detached) => State::On,
            State::Connected if matches!(event, CellularEvent::Ota) => State::Ota,
            State::Connected if matches!(event, CellularEvent::DataInactive) => State::Registered,
            State::Rfoff if matches!(event, CellularEvent::PwrOn) => State::Rfoff,
            State::Rfoff if matches!(event, CellularEvent::PwrOff) => State::Off,
            State::Rfoff if matches!(event, CellularEvent::RfOn) => State::On,
            State::Ota if matches!(event, CellularEvent::OtaDone) => State::Off,
            State::Ota if matches!(event, CellularEvent::PwrOff) => State::Off,
            _ => {
                defmt::error!("Wrong event received {:?}", event);
                return;
            }
        };

        self.set_state(new_state);
    }

    #[allow(dead_code)]
    pub(crate) fn set_max_retry_attempts(&mut self, max_retry_attempts: u8) {
        self.max_retry_attempts = max_retry_attempts;
    }

    pub(crate) fn reset(&mut self) {
        self.max_retry_attempts = 10;
        self.retry_count = None;
    }

    pub(crate) const fn get_state(self) -> State {
        self.inner
    }

    pub(crate) fn set_state(&mut self, state: State) {
        defmt::debug!("State transition: {:?} -> {:?}", self.inner, state);
        // Reset the max attempts on any state transition
        self.reset();
        self.inner = state;
    }

    pub(crate) const fn is_retry(self) -> bool {
        self.retry_count.is_some()
    }

    pub(crate) fn retry_or_fail<CNT>(&mut self, timer: &mut CNT) -> nb::Error<Error>
    where
        CNT: CountDown,
        CNT::Time: From<u32>,
    {
        // Handle retry based on exponential backoff here
        match self.retry_count {
            Some(cnt) if cnt >= self.max_retry_attempts => {
                // Max attempts reached! Bail with a timeout error
                return nb::Error::Other(Error::StateTimeout);
            }
            _ => {}
        }

        let cnt = self.retry_count.unwrap_or_default();

        // FIXME: Change to a poor-mans exponential
        let backoff_time = (u32::from(cnt) + 1) * 1000;

        if timer.try_start(backoff_time).is_err() {
            defmt::error!("Failed to start retry_timer!!");
            return nb::Error::Other(Error::_Unknown);
        }

        defmt::warn!(
            "[RETRY] current attempt: {:u8}, retrying state({:?}) in {:u32} ms...",
            cnt,
            self.inner,
            backoff_time
        );

        self.retry_count = Some(cnt + 1);
        nb::Error::WouldBlock
    }
}

pub struct RegistrationParams {
    reg_type: RegType,
    pub(crate) status: RegistrationStatus,
    act: RatAct,

    cell_id: Option<String<consts::U8>>,
    lac: Option<String<consts::U4>>,
    // active_time: Option<u16>,
    // periodic_tau: Option<u16>,
}

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum RegType {
    Creg,
    Cgreg,
    Cereg,
    Unknown,
}

impl From<RadioAccessNetwork> for RegType {
    fn from(ran: RadioAccessNetwork) -> Self {
        match ran {
            RadioAccessNetwork::UnknownUnused => RegType::Unknown,
            RadioAccessNetwork::Geran => RegType::Creg,
            RadioAccessNetwork::Utran => RegType::Cgreg,
            RadioAccessNetwork::Eutran => RegType::Cereg,
        }
    }
}

impl From<RegType> for RadioAccessNetwork {
    fn from(regtype: RegType) -> Self {
        match regtype {
            RegType::Unknown => RadioAccessNetwork::UnknownUnused,
            RegType::Creg => RadioAccessNetwork::Geran,
            RegType::Cgreg => RadioAccessNetwork::Utran,
            RegType::Cereg => RadioAccessNetwork::Eutran,
        }
    }
}

#[derive(Debug, Default)]
pub struct ServiceStatus {
    /// Radio Access Technology (RAT)
    pub rat: RatAct,

    /// Network registration mode (auto/manual etc.) currently selected.
    pub network_registration_mode: OperatorSelectionMode,

    /// CS (Circuit Switched) registration status (registered/searching/roaming etc.).
    pub cs_reg_status: RegistrationStatus,
    /// PS (Packet Switched) registration status (registered/searching/roaming etc.).
    pub ps_reg_status: RegistrationStatus,

    /// Registered network operator name.
    pub operator: Option<OperatorNameFormat>,

    /// CS Reject Type. 0 - 3GPP specific Reject Cause. 1 - Manufacture specific
    pub cs_reject_type: Option<u8>,
    /// Reason why the CS (Circuit Switched) registration attempt was rejected
    pub cs_reject_cause: Option<u8>,
    /// PS Reject Type. 0 - 3GPP specific Reject Cause. 1 - Manufacture specific
    pub ps_reject_type: Option<u8>,
    /// Reason why the PS (Packet Switched) registration attempt was rejected
    pub ps_reject_cause: Option<u8>,
}

pub struct NetworkStatus {
    /// Radio Access Technology (RAT)
    rat: RatAct,

    /// CS (Circuit Switched) registration status (registered/searching/roaming etc.).
    pub(crate) cs_reg_status: RegistrationStatus,
    /// PS (Packet Switched) registration status (registered/searching/roaming etc.).
    pub(crate) ps_reg_status: RegistrationStatus,

    /// CS Reject Type. 0 - 3GPP specific Reject Cause. 1 - Manufacture specific
    _cs_reject_type: u8,
    /// Reason why the CS (Circuit Switched) registration attempt was rejected
    _cs_reject_cause: u8,
    /// PS Reject Type. 0 - 3GPP specific Reject Cause. 1 - Manufacture specific
    _ps_reject_type: u8,
    /// Reason why the PS (Packet Switched) registration attempt was rejected
    _ps_reject_cause: u8,

    /// Registered network operator cell Id.
    cell_id: Option<String<consts::U8>>,
    /// Registered network operator Location Area Code.
    lac: Option<String<consts::U4>>,
    /// Registered network operator Routing Area Code.
    // rac: u8,
    /// Registered network operator Tracking Area Code.
    // tac: u8,
    pub events: Queue<Event, consts::U20, u8>,
}

impl From<&mut NetworkStatus> for ServiceStatus {
    fn from(ns: &mut NetworkStatus) -> Self {
        ServiceStatus {
            rat: ns.rat,
            network_registration_mode: OperatorSelectionMode::Unknown,
            cs_reg_status: ns.cs_reg_status,
            ps_reg_status: ns.ps_reg_status,
            operator: None,
            cs_reject_type: None,
            cs_reject_cause: None,
            ps_reject_type: None,
            ps_reject_cause: None,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    RadioAccessTechnologyChanged(RadioAccessNetwork, RatAct),
    RegistrationStatusChanged(RegType, ServiceStatus),
    CellIDChanged(Option<String<consts::U8>>),

    /// trigger: an module is powered on
    PwrOn,
    /// trigger: cellular module is powered off
    PwrOff,
    /// trigger: airplane mode off
    RfOn,
    /// trigger: airplane mode on
    RfOff,
    /// trigger: attached to a network
    Attached,
    /// trigger: detached from a network
    Detached,
    /// trigger: data connect is active
    DataActive,
    /// trigger: data connection is inactive
    DataInactive,
    /// trigger: OTA starts
    Ota,
    /// trigger: OTA finishes
    OtaDone,
}

impl Default for RegistrationParams {
    fn default() -> Self {
        Self {
            act: RatAct::Unknown,
            reg_type: RegType::Unknown,
            status: RegistrationStatus::StatusNotAvailable,
            cell_id: None,
            lac: None,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl NetworkStatus {
    pub fn new() -> Self {
        Self {
            events: Queue::u8(),
            rat: RatAct::default(),
            cs_reg_status: RegistrationStatus::default(),
            ps_reg_status: RegistrationStatus::default(),
            _cs_reject_type: 0,
            _cs_reject_cause: 0,
            _ps_reject_type: 0,
            _ps_reject_cause: 0,
            cell_id: None,
            lac: None,
        }
    }
    pub fn push_event(&mut self, event: Event) {
        self.events.enqueue(event).ok();
    }

    pub fn compare_and_set(&mut self, new_params: RegistrationParams) {
        match new_params.reg_type {
            RegType::Creg if self.cs_reg_status != new_params.status => {
                self.cs_reg_status = new_params.status;
                let status = self.into();
                self.push_event(Event::RegistrationStatusChanged(
                    new_params.reg_type,
                    status,
                ));
            }
            RegType::Cgreg | RegType::Cereg if self.ps_reg_status != new_params.status => {
                self.ps_reg_status = new_params.status;
                let status = self.into();
                self.push_event(Event::RegistrationStatusChanged(
                    new_params.reg_type,
                    status,
                ));
            }
            RegType::Unknown => {
                defmt::error!("unknown reg type");
                return;
            }
            _ => {
                return;
            }
        }

        if self.rat != new_params.act {
            self.push_event(Event::RadioAccessTechnologyChanged(
                new_params.reg_type.into(),
                self.rat,
            ));
        }
        if new_params.cell_id.is_some() && self.cell_id != new_params.cell_id {
            self.cell_id = new_params.cell_id.clone();
            self.lac = new_params.lac;
            self.push_event(Event::CellIDChanged(new_params.cell_id));
        }
    }
}

impl From<NetworkRegistration> for RegistrationParams {
    fn from(v: NetworkRegistration) -> Self {
        Self {
            act: RatAct::Gsm,
            reg_type: RegType::Creg,
            status: v.stat.into(),
            cell_id: None,
            lac: None,
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<NetworkRegistrationStatus> for RegistrationParams {
    fn from(v: NetworkRegistrationStatus) -> Self {
        Self {
            act: RatAct::Gsm,
            reg_type: RegType::Creg,
            status: v.stat.into(),
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
            act: v.act.unwrap_or(RatAct::Unknown),
            reg_type: RegType::Cgreg,
            status: v.stat.into(),
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
            reg_type: RegType::Cgreg,
            status: v.stat.into(),
            cell_id: v.ci,
            lac: v.lac,
            act: v.act.unwrap_or(RatAct::Unknown),
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<EPSNetworkRegistration> for RegistrationParams {
    fn from(v: EPSNetworkRegistration) -> Self {
        Self {
            reg_type: RegType::Cereg,
            status: v.stat.into(),
            cell_id: v.ci,
            lac: v.tac,
            act: v.act.unwrap_or(RatAct::Unknown),
            // active_time: None,
            // periodic_tau: None,
        }
    }
}

impl From<EPSNetworkRegistrationStatus> for RegistrationParams {
    fn from(v: EPSNetworkRegistrationStatus) -> Self {
        Self {
            reg_type: RegType::Cereg,
            status: v.stat.into(),
            cell_id: v.ci,
            lac: v.tac,
            act: v.act.unwrap_or(RatAct::Unknown),
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

impl Default for RegistrationStatus {
    fn default() -> Self {
        Self::StatusNotAvailable
    }
}

impl RegistrationStatus {
    pub const fn is_registered(self) -> bool {
        matches!(self, Self::RegisteredHomeNetwork | Self::RegisteredRoaming)
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
