use atat::AtatClient;
use core::cell::{Cell, RefCell};
use embedded_hal::{
    blocking::delay::DelayMs,
    digital::{InputPin, OutputPin},
};
use heapless::{consts, ArrayLength, Bucket, Pos, PowerOfTwo, String};

use crate::{
    command::{
        control::{types::*, *},
        mobile_control::{types::*, *},
        system_features::{types::*, *},
        Urc, *,
    },
    error::Error,
    gprs::{APNInfo, Apn},
    module_cfg::constants::{CONTEXT_ID, PROFILE_ID},
    socket::SocketSet,
};
use network_service::{
    types::{NetworkRegistrationStat, NetworkRegistrationUrcConfig, OperatorSelectionMode},
    GetOperatorSelection, SetNetworkRegistrationStatus, SetOperatorSelection,
};
use psn::{
    types::{
        EPSNetworkRegistrationStat, EPSNetworkRegistrationUrcConfig, GPRSAttachedState,
        GPRSNetworkRegistrationStat, GPRSNetworkRegistrationUrcConfig, PDPContextStatus,
        PacketSwitchedAction,
    },
    GetEPSNetworkRegistrationStatus, GetGPRSAttached, GetGPRSNetworkRegistrationStatus,
    SetEPSNetworkRegistrationStatus, SetGPRSNetworkRegistrationStatus, SetPDPContextDefinition,
    SetPDPContextState, SetPacketSwitchedAction, SetPacketSwitchedConfig,
};
use sms::{types::MessageWaitingMode, SetMessageWaitingIndication};

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum State {
    Init,
    PowerOn,
    DeviceReady,
    SimPin,
    SignalQuality,
    RegisteringNetwork,
    AttachingNetwork,
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
        self.0
            .iter()
            .enumerate()
            .any(|(_, &x)| x == NetworkStatus::Registered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum NetworkStatus {
    Unknown,
    TemporaryNetworkBarring,
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

pub struct NoPin;

impl InputPin for NoPin {
    type Error = core::convert::Infallible;

    fn try_is_high(&self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn try_is_low(&self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl OutputPin for NoPin {
    type Error = core::convert::Infallible;

    fn try_set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn try_set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Config<RST, DTR, PWR, VINT> {
    rst_pin: Option<RST>,
    dtr_pin: Option<DTR>,
    pwr_pin: Option<PWR>,
    vint_pin: Option<VINT>,
    baud_rate: u32,
    pub(crate) hex_mode: bool,
    flow_control: bool,
    pub(crate) apn_info: APNInfo,
    pin: String<consts::U4>,
}

impl<RST, DTR, PWR, VINT> Default for Config<RST, DTR, PWR, VINT> {
    fn default() -> Self {
        Config {
            rst_pin: None,
            dtr_pin: None,
            pwr_pin: None,
            vint_pin: None,
            baud_rate: 115_200_u32,
            hex_mode: true,
            flow_control: false,
            apn_info: APNInfo::default(),
            pin: String::new(),
        }
    }
}

impl<RST, DTR, PWR, VINT> Config<RST, DTR, PWR, VINT>
where
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
    pub fn new(pin: &str) -> Self {
        Config {
            rst_pin: None,
            dtr_pin: None,
            pwr_pin: None,
            vint_pin: None,
            baud_rate: 115_200_u32,
            hex_mode: true,
            flow_control: false,
            apn_info: APNInfo::default(),
            pin: String::from(pin),
        }
    }

    pub fn with_rst(self, rst_pin: RST) -> Self {
        Config {
            rst_pin: Some(rst_pin),
            ..self
        }
    }

    pub fn with_pwr(self, pwr_pin: PWR) -> Self {
        Config {
            pwr_pin: Some(pwr_pin),
            ..self
        }
    }

    pub fn with_dtr(self, dtr_pin: DTR) -> Self {
        Config {
            dtr_pin: Some(dtr_pin),
            ..self
        }
    }

    pub fn with_vint(self, vint_pin: VINT) -> Self {
        Config {
            vint_pin: Some(vint_pin),
            ..self
        }
    }

    pub fn baud_rate<B: Into<u32>>(self, baud_rate: B) -> Self {
        // FIXME: Validate baudrates

        Config {
            baud_rate: baud_rate.into(),
            ..self
        }
    }

    pub fn with_flow_control(self) -> Self {
        Config {
            flow_control: true,
            ..self
        }
    }

    pub fn with_apn_info(self, apn_info: APNInfo) -> Self {
        Config { apn_info, ..self }
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }
}

pub struct GsmClient<C, DLY, N, L, RST = NoPin, DTR = NoPin, PWR = NoPin, VINT = NoPin>
where
    C: AtatClient,
    DLY: DelayMs<u32>,
    N: 'static
        + ArrayLength<Option<crate::sockets::SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    initialized: Cell<bool>,
    pub(crate) state: Cell<State>,
    pub(crate) config: Config<RST, DTR, PWR, VINT>,
    pub(crate) delay: RefCell<DLY>,
    pub(crate) network_status: RefCell<RANStatus>,
    pub(crate) client: RefCell<C>,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: RefCell<&'static mut SocketSet<N, L>>,
}

impl<C, DLY, N, L, RST, DTR, PWR, VINT> GsmClient<C, DLY, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    DLY: DelayMs<u32>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>
        + PowerOfTwo,
    L: ArrayLength<u8>,
{
    pub fn new(
        client: C,
        socket_set: &'static mut SocketSet<N, L>,
        delay: DLY,
        config: Config<RST, DTR, PWR, VINT>,
    ) -> Self {
        GsmClient {
            config,
            delay: RefCell::new(delay),
            network_status: RefCell::new(RANStatus::new()),
            initialized: Cell::new(false),
            state: Cell::new(State::Init),
            client: RefCell::new(client),
            sockets: RefCell::new(socket_set),
        }
    }

    /// Initialize a new ublox device to a known state (restart, wait for
    /// startup, set RS232 settings, gpio settings, etc.)
    pub fn initialize(&mut self, leave_pwr_alone: bool) -> Result<(), Error> {
        if !self.initialized.get() {
            defmt::info!(
                "Initialising with PWR_ON pin: {:bool} and VInt pin: {:bool}",
                self.config.pwr_pin.is_some(),
                self.config.vint_pin.is_some()
            );

            match self.config.pwr_pin {
                Some(ref mut pwr) if !leave_pwr_alone => {
                    pwr.try_set_high().ok();
                }
                _ => {}
            }

            self.initialized.set(true);
        }

        self.power_on()?;

        Ok(())
    }

    fn power_on(&mut self) -> Result<(), Error> {
        let vint_value = match self.config.vint_pin {
            Some(ref _vint) => false,
            _ => false,
        };

        if vint_value || self.is_alive(1).is_ok() {
            defmt::debug!("powering on, module is already on, flushing config...");
            self.configure()?;
        } else {
            defmt::debug!("powering on.");
            if let Some(ref mut pwr) = self.config.pwr_pin {
                pwr.try_set_low().ok();
                self.delay
                    .try_borrow_mut()?
                    .try_delay_ms(crate::module_cfg::constants::PWR_ON_PULL_TIME_MS)
                    .map_err(|_| Error::Busy)?;
                pwr.try_set_high().ok();
            } else {
                // Software restart
                self.restart()?;
            }
            self.delay
                .try_borrow_mut()?
                .try_delay_ms(crate::module_cfg::constants::BOOT_WAIT_TIME_MS)
                .map_err(|_| Error::Busy)?;
            self.is_alive(10)?;
            self.configure()?;
        }
        Ok(())
    }

    /// Check that the cellular module is alive.
    ///
    /// See if the cellular module is responding at the AT interface by poking
    /// it with "AT" up to "attempts" times, waiting 1 second for an "OK"
    /// response each time
    #[inline]
    fn is_alive(&self, attempts: u8) -> Result<(), Error> {
        let mut error = Error::BaudDetection;
        for _ in 0..attempts {
            match self.client.try_borrow_mut()?.send(&AT) {
                Ok(_) => {
                    return Ok(());
                }
                Err(nb::Error::Other(e)) => error = e.into(),
                Err(nb::Error::WouldBlock) => {}
            };
        }
        Err(error)
    }

    fn configure(&self) -> Result<(), Error> {
        if self.config.baud_rate > 230_400_u32 {
            // Needs a way to reconfigure uart baud rate temporarily
            // Relevant issue: https://github.com/rust-embedded/embedded-hal/issues/79
            return Err(Error::_Unknown);

            // self.send_internal(
            //     &SetDataRate {
            //         rate: BaudRate::B115200,
            //     },
            //     true,
            // )?;

            // NOTE: On the UART AT interface, after the reception of the "OK" result code for the +IPR command, the DTE
            // shall wait for at least 100 ms before issuing a new AT command; this is to guarantee a proper baud rate
            // reconfiguration.

            // UART end
            // delay(100);
            // UART begin(self.config.baud_rate)

            // self.is_alive()?;
        }

        // Extended errors on
        self.send_internal(
            &SetReportMobileTerminationError {
                n: TerminationErrorMode::Disabled,
            },
            false,
        )?;

        // DCD circuit (109) changes in accordance with the carrier
        self.send_internal(
            &SetCircuit109Behaviour {
                value: Circuit109Behaviour::ChangesWithCarrier,
            },
            false,
        )?;

        // Ignore changes to DTR
        self.send_internal(
            &SetCircuit108Behaviour {
                value: Circuit108Behaviour::Ignore,
            },
            false,
        )?;

        // TODO: switch off UART power saving until it is integrated into this API
        self.send_internal(
            &SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
            },
            false,
        )?;

        // self.send_internal(
        //     &mobile_control::SetAutomaticTimezoneUpdate {
        //         on_off: AutomaticTimezone::EnabledLocal,
        //     },
        //     false,
        // )?;

        // self.send_internal(&general::IdentificationInformation { n: 9 }, true)?;

        // Stay in airplane mode until commanded to connect
        self.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::AirplaneMode,
                rst: None,
            },
            false,
        )?;

        if self.config.flow_control {
            self.send_internal(
                &SetFlowControl {
                    value: FlowControl::RtsCts,
                },
                false,
            )?;
        } else {
            self.send_internal(
                &SetFlowControl {
                    value: FlowControl::Disabled,
                },
                false,
            )?;
        }

        self.send_internal(
            &SetMessageWaitingIndication {
                mode: MessageWaitingMode::Disabled,
            },
            false,
        )?;

        Ok(())
    }

    #[inline]
    fn restart(&self) -> Result<(), Error> {
        self.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::SilentReset,
                rst: None,
            },
            false,
        )?;
        Ok(())
    }

    pub fn spin(&self) -> Result<(), Error> {
        self.handle_urc().ok();

        // TODO: check registration and re-register if needed

        if self.network_status.borrow().is_registered() {
            self.sockets
                .try_borrow_mut()?
                .iter_mut()
                .try_for_each(|(_, socket)| self.socket_ingress(socket))?;
        }

        Ok(())
    }

    fn handle_urc(&self) -> Result<(), Error> {
        let urc = self.client.try_borrow_mut()?.check_urc::<Urc>();

        match urc {
            Some(Urc::ExtendedPSNetworkRegistration(psn::urc::ExtendedPSNetworkRegistration {
                state,
            })) => {
                defmt::info!("[URC] ExtendedPSNetworkRegistration {:?}", state);
                Ok(())
            }
            Some(Urc::GPRSNetworkRegistration(psn::urc::GPRSNetworkRegistration { stat })) => {
                defmt::info!("[URC] GPRSNetworkRegistration {:?}", stat);
                Ok(self
                    .network_status
                    .try_borrow_mut()?
                    .set(RadioAccessNetwork::Utran, stat.into()))
            }
            Some(Urc::EPSNetworkRegistration(psn::urc::EPSNetworkRegistration { stat })) => {
                defmt::info!("[URC] EPSNetworkRegistration {:?}", stat);
                Ok(self
                    .network_status
                    .try_borrow_mut()?
                    .set(RadioAccessNetwork::Eutran, stat.into()))
            }
            Some(Urc::NetworkRegistration(network_service::urc::NetworkRegistration { stat })) => {
                defmt::info!("[URC] NetworkRegistration {:?}", stat);
                Ok(self
                    .network_status
                    .try_borrow_mut()?
                    .set(RadioAccessNetwork::Geran, stat.into()))
            }
            Some(Urc::MessageWaitingIndication(_)) => {
                defmt::info!("[URC] MessageWaitingIndication");
                Ok(())
            }
            Some(Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket })) => {
                defmt::info!("[URC] SocketClosed {:u8}", socket.0);
                self.sockets.try_borrow_mut()?.remove(socket)?;
                Ok(())
            }
            Some(Urc::DataConnectionActivated(psn::urc::DataConnectionActivated { result })) => {
                defmt::info!("[URC] DataConnectionActivated {:u8}", result);
                Ok(())
            }
            Some(Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                profile_id,
            })) => {
                defmt::info!("[URC] DataConnectionDeactivated {:u8}", profile_id);
                Ok(())
            }
            Some(Urc::SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable {
                socket,
                length,
            }))
            | Some(Urc::SocketDataAvailableUDP(ip_transport_layer::urc::SocketDataAvailable {
                socket,
                length,
            })) => {
                defmt::trace!(
                    "[Socket({:u8})] {:u16} bytes available",
                    socket.0,
                    length as u16
                );

                Ok(self
                    .sockets
                    .try_borrow_mut()?
                    .iter_mut()
                    .find(|(handle, _)| *handle == socket)
                    .ok_or(Error::SocketNotFound)?
                    .1
                    .set_available_data(length))
            }
            None => Ok(()),
        }
    }

    pub fn nwk_registration(&self) -> Result<(), Error> {
        if !self.network_status.try_borrow()?.is_registered() {
            self.send_internal(
                &SetNetworkRegistrationStatus {
                    n: NetworkRegistrationUrcConfig::UrcDisabled,
                },
                true,
            )?;

            self.send_internal(
                &SetGPRSNetworkRegistrationStatus {
                    n: GPRSNetworkRegistrationUrcConfig::UrcEnabled,
                },
                true,
            )?;

            self.send_internal(
                &SetEPSNetworkRegistrationStatus {
                    n: EPSNetworkRegistrationUrcConfig::UrcEnabled,
                },
                true,
            )?;

            let cops = self.send_internal(&GetOperatorSelection, true)?;

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

            defmt::info!("Done preparing connect..");
        }
        Ok(())
    }

    pub fn try_connect(&self, apn_info: &APNInfo) -> Result<(), Error> {
        defmt::info!("Attempting connect..");
        // Set up context definition
        match apn_info.apn {
            Apn::Given(ref apn) => {
                self.send_internal(
                    &SetPDPContextDefinition {
                        cid: CONTEXT_ID,
                        PDP_type: "IP",
                        apn: apn.as_str(),
                    },
                    true,
                )?;
            }
            Apn::Automatic => {
                // Lookup APN in DB!
            }
        }

        // Set up authentication mode, if required
        if apn_info.user_name.is_some() || apn_info.password.is_some() {
            // TODO: `AT+UAUTHREQ=` here
            // {
            //     cid: CONTEXT_ID,
            //     auth_type: AuthType::Automatic,
            //     username: apn_info.user_name.unwrap_or_default(),
            //     password: apn_info.user_name.unwrap_or_default(),
            // }
        }

        // Now come out of airplane mode and try to register
        self.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            },
            true,
        )?;

        // Wait for registration to succeed
        let mut i: u8 = 0;
        while i < 180 && !self.network_status.try_borrow()?.is_registered() {
            let (ran, status) = match i % 2 {
                // 0 => self
                //     .send_internal(&GetNetworkRegistrationStatus, true)
                //     .map(|s| (RadioAccessNetwork::Geran, s.stat.into()))?,
                0 => self
                    .send_internal(&GetGPRSNetworkRegistrationStatus, true)
                    .map(|s| (RadioAccessNetwork::Utran, s.stat.into()))?,
                _ => self
                    .send_internal(&GetEPSNetworkRegistrationStatus, true)
                    .map(|s| (RadioAccessNetwork::Eutran, s.stat.into()))?,
            };

            self.network_status.try_borrow_mut()?.set(ran, status);

            // delay 1000 ms
            self.delay
                .try_borrow_mut()?
                .try_delay_ms(1000)
                .map_err(|_| Error::Busy)?;

            i += 1;
        }

        if self.network_status.try_borrow()?.is_registered() {
            // Network is registered!

            // Now, technically speaking, EUTRAN should be good to go, PDP context
            // and everything, and we should only have to activate a PDP context on
            // GERAN.  However, for reasons I don't understand, SARA-R4 can be
            // registered but not attached (i.e. AT+CGATT returns 0) on both RATs
            // (unh?).  Phil Ware, who knows about these things, always goes through
            // (a) register, (b) wait for AT+CGATT to return 1 and then (c) check
            // that a context is active with AT+CGACT (even for EUTRAN). Since this
            // sequence works for both RANs, it's best to be consistent. Wait for
            // AT+CGATT to return 1 SARA R4/N4 AT Command Manual UBX-17003787,
            // section 13.5
            let mut attempt: u8 = 0;
            while self.send_internal(&GetGPRSAttached, true)?.state != GPRSAttachedState::Attached
                && attempt < 10
            {
                // delay 1000 ms
                self.delay
                    .try_borrow_mut()?
                    .try_delay_ms(1000)
                    .map_err(|_| Error::Busy)?;

                attempt += 1;
            }

            if attempt < 10 {
                // Activate a PDP context
                //TODO: Check AT+CGACT? to verify that `CONTEXT_ID` is active

                // If not active, help it on its way.
                self.delay
                    .try_borrow_mut()?
                    .try_delay_ms(1000)
                    .map_err(|_| Error::Busy)?;

                self.send_internal(
                    &SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: Some(CONTEXT_ID),
                    },
                    true,
                )?;

                // PDP context active
                self.send_internal(
                    &SetPacketSwitchedConfig {
                        profile_id: PROFILE_ID,
                        param: psn::types::PacketSwitchedParam::MapProfile(CONTEXT_ID),
                    },
                    true,
                )?;

                for _ in 0..10 {
                    match self.send_internal(
                        &SetPacketSwitchedAction {
                            profile_id: PROFILE_ID,
                            action: PacketSwitchedAction::Activate,
                        },
                        true,
                    ) {
                        Ok(_) => return Ok(()),
                        Err(_) => {
                            defmt::warn!("Failed UPSDA!");
                        }
                    }

                    self.delay
                        .try_borrow_mut()?
                        .try_delay_ms(1000)
                        .map_err(|_| Error::Busy)?;
                }
            }
        }

        self.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::AirplaneMode,
                rst: None,
            },
            true,
        )?;

        Err(Error::_Unknown)
    }

    #[inline]
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

        self.client
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
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        if !self.initialized.get() {
            return Err(Error::Uninitialized);
        }

        self.send_internal(cmd, true)
    }
}
