use crate::command::{
    network_service::{
        responses::NetworkRegistrationStatus,
        types::{NetworkRegistrationStat, RatAct},
        urc::NetworkRegistration,
    },
    psn::{
        responses::{EPSNetworkRegistrationStatus, GPRSNetworkRegistrationStatus},
        types::{EPSNetworkRegistrationStat, GPRSNetworkRegistrationStat},
        urc::{EPSNetworkRegistration, GPRSNetworkRegistration},
    },
};
use embassy_time::{Duration, Instant};
use heapless::String;

#[derive(Debug, Clone, Default)]
pub struct CellularRegistrationStatus {
    status: Status,
    updated: Option<Instant>,
    started: Option<Instant>,
}

impl CellularRegistrationStatus {
    pub const fn new() -> Self {
        Self {
            status: Status::None,
            updated: None,
            started: None,
        }
    }

    pub fn duration(&self, ts: Instant) -> Duration {
        self.started
            .and_then(|started| ts.checked_duration_since(started))
            .unwrap_or_else(|| Duration::from_millis(0))
    }

    pub fn started(&self) -> Option<Instant> {
        self.started
    }

    pub fn updated(&self) -> Option<Instant> {
        self.updated
    }

    pub fn reset(&mut self) {
        self.status = Status::None;
        self.updated = None;
        self.started = None;
    }

    pub fn get_status(&self) -> Status {
        self.status
    }

    pub fn set_status(&mut self, stat: Status) {
        let ts = Instant::now();
        if self.status != stat {
            self.status = stat;
            self.started = Some(ts);
        }
        self.updated = Some(ts);
    }

    pub fn registered(&self) -> bool {
        matches!(self.status, Status::Home | Status::Roaming)
    }

    pub fn sticky(&self) -> bool {
        self.updated.is_some() && self.updated != self.started
    }
}

impl From<u8> for Status {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::NotRegistering,
            1 => Self::Home,
            2 => Self::Searching,
            3 => Self::Denied,
            4 => Self::OutOfCoverage,
            5 => Self::Roaming,
            _ => Self::None,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Status {
    #[default]
    None,
    NotRegistering,
    Home,
    Searching,
    Denied,
    OutOfCoverage,
    Roaming,
}

/// Convert the 3GPP registration status from a CREG URC to [`RegistrationStatus`].
impl From<NetworkRegistrationStat> for Status {
    fn from(v: NetworkRegistrationStat) -> Self {
        Self::from(v as u8)
    }
}

/// Convert the 3GPP registration status from a CGREG URC to [`RegistrationStatus`].
impl From<GPRSNetworkRegistrationStat> for Status {
    fn from(v: GPRSNetworkRegistrationStat) -> Self {
        Self::from(v as u8)
    }
}

/// Convert the 3GPP registration status from a CEREG URC to [`RegistrationStatus`].
impl From<EPSNetworkRegistrationStat> for Status {
    fn from(v: EPSNetworkRegistrationStat) -> Self {
        Self::from(v as u8)
    }
}

#[cfg(not(feature = "use-upsd-context-activation"))]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ProfileState {
    #[default]
    Unknown,
    ShouldBeUp,
    RequiresReactivation,
    ShouldBeDown,
}

#[derive(Debug, Default)]
pub struct RegistrationParams {
    reg_type: RegType,
    pub(crate) status: Status,
    act: RatAct,

    cell_id: Option<String<8>>,
    lac: Option<String<4>>,
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RegType {
    Creg,
    Cgreg,
    Cereg,
    #[default]
    Unknown,
}

impl From<RadioAccessNetwork> for RegType {
    fn from(ran: RadioAccessNetwork) -> Self {
        match ran {
            RadioAccessNetwork::UnknownUnused => Self::Unknown,
            RadioAccessNetwork::Geran => Self::Creg,
            RadioAccessNetwork::Utran => Self::Cgreg,
            RadioAccessNetwork::Eutran => Self::Cereg,
        }
    }
}

impl From<RegType> for RadioAccessNetwork {
    fn from(regtype: RegType) -> Self {
        match regtype {
            RegType::Unknown => Self::UnknownUnused,
            RegType::Creg => Self::Geran,
            RegType::Cgreg => Self::Utran,
            RegType::Cereg => Self::Eutran,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CellularGlobalIdentity {
    /// Registered network operator cell Id.
    cell_id: Option<String<8>>,
    /// Registered network operator Location Area Code.
    lac: Option<String<4>>,
    // Registered network operator Routing Area Code.
    // rac: u8,
    // Registered network operator Tracking Area Code.
    // tac: u8,
}

impl CellularGlobalIdentity {
    pub const fn new() -> Self {
        Self {
            cell_id: None,
            lac: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegistrationState {
    /// CSD (Circuit Switched Data) registration status (registered/searching/roaming etc.).
    pub(crate) csd: CellularRegistrationStatus,
    /// PSD (Packet Switched Data) registration status (registered/searching/roaming etc.).
    pub(crate) psd: CellularRegistrationStatus,
    /// EPS (Evolved Packet Switched) registration status (registered/searching/roaming etc.).
    pub(crate) eps: CellularRegistrationStatus,

    pub(crate) cgi: CellularGlobalIdentity,

    #[cfg(not(feature = "use-upsd-context-activation"))]
    pub(crate) profile_state: ProfileState,
}

impl Default for RegistrationState {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistrationState {
    pub const fn new() -> Self {
        Self {
            csd: CellularRegistrationStatus::new(),
            psd: CellularRegistrationStatus::new(),
            eps: CellularRegistrationStatus::new(),
            cgi: CellularGlobalIdentity::new(),

            #[cfg(not(feature = "use-upsd-context-activation"))]
            profile_state: ProfileState::Unknown,
        }
    }

    /// Determine if a given cellular network status value means that we're
    /// registered with the network.
    pub fn is_registered(&self) -> bool {
        // If PSD or EPS are registered, we are connected!
        self.psd.registered() || self.eps.registered()
    }

    pub fn reset(&mut self) {
        self.csd.reset();
        self.psd.reset();
        self.eps.reset();
    }

    pub fn compare_and_set(&mut self, new_params: RegistrationParams) {
        match new_params.reg_type {
            RegType::Creg => {
                self.csd.set_status(new_params.status);
            }
            RegType::Cgreg => {
                self.psd.set_status(new_params.status);
            }
            RegType::Cereg => {
                self.eps.set_status(new_params.status);
            }
            RegType::Unknown => {
                error!("unknown reg type");
                return;
            }
        }

        // Update Cellular Global Identity
        if new_params.cell_id.is_some() && self.cgi.cell_id != new_params.cell_id {
            self.cgi.cell_id = new_params.cell_id.clone();
            self.cgi.lac = new_params.lac;
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
        }
    }
}

impl From<GPRSNetworkRegistration> for RegistrationParams {
    fn from(v: GPRSNetworkRegistration) -> Self {
        Self {
            act: v.act.unwrap_or(RatAct::GsmGprsEdge),
            reg_type: RegType::Cgreg,
            status: v.stat.into(),
            cell_id: v.ci,
            lac: v.lac,
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
            act: v.act.unwrap_or(RatAct::GsmGprsEdge),
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
            act: v.act.unwrap_or(RatAct::Lte),
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
            act: v.act.unwrap_or(RatAct::Lte),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
