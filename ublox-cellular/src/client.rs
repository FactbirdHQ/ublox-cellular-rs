use atat::AtatClient;
use core::cell::RefCell;
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
    gprs::APNInfo,
    module_cfg::constants::{CONTEXT_ID, PROFILE_ID},
    socket::SocketSet,
};
use ip_transport_layer::{types::HexMode, SetHexMode};
use network_service::{
    types::{NetworkRegistrationStat, NetworkRegistrationUrcConfig, OperatorSelectionMode},
    GetOperatorSelection, SetNetworkRegistrationStatus, SetOperatorSelection,
};
use psn::{
    responses::GPRSAttached,
    types::{
        EPSNetworkRegistrationStat, EPSNetworkRegistrationUrcConfig, GPRSAttachedState,
        GPRSNetworkRegistrationStat, GPRSNetworkRegistrationUrcConfig, PDPContextStatus,
        PacketSwitchedAction,
    },
    GetGPRSAttached, SetEPSNetworkRegistrationStatus, SetGPRSAttached,
    SetGPRSNetworkRegistrationStatus, SetPDPContextState, SetPacketSwitchedAction,
    SetPacketSwitchedConfig,
};
use sms::{types::MessageWaitingMode, SetMessageWaitingIndication};

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum State {
    Init,
    PowerOn,
    Configure,
    SimPin,
    SignalQuality,
    RegisteringNetwork,
    AttachingNetwork,
    ActivatingContext,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct StateMachine {
    timeout: Option<u32>,
    retry_count: u8,
    inner: State,
}

impl StateMachine {
    fn new() -> Self {
        StateMachine {
            timeout: None,
            retry_count: 0,
            inner: State::Init,
        }
    }

    fn get_state(&self) -> State {
        self.inner
    }

    fn set_state(&mut self, new_state: State) {
        defmt::debug!("State transition: {:?} -> {:?}", self.inner, new_state);
        self.inner = new_state;
    }

    fn is_retry(&self) -> bool {
        self.retry_count > 0
    }

    fn retry_or_fail(&mut self) -> nb::Result<(), Error> {
        // Handle retry based on exponential backoff here

        if self.retry_count < 10 {
            self.retry_count += 1;
            Err(nb::Error::WouldBlock)
        } else {
            Err(nb::Error::Other(Error::StateTimeout))
        }
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
    pub(crate) fsm: RefCell<StateMachine>,
    pub(crate) config: RefCell<Config<RST, DTR, PWR, VINT>>,
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
            config: RefCell::new(config),
            delay: RefCell::new(delay),
            network_status: RefCell::new(RANStatus::new()),
            fsm: RefCell::new(StateMachine::new()),
            client: RefCell::new(client),
            sockets: RefCell::new(socket_set),
        }
    }

    fn initialize(&self, leave_pwr_alone: bool) -> Result<(), Error> {
        defmt::info!(
            "Initialising with PWR_ON pin: {:bool} and VInt pin: {:bool}",
            self.config.try_borrow()?.pwr_pin.is_some(),
            self.config.try_borrow()?.vint_pin.is_some()
        );

        match self.config.try_borrow_mut()?.pwr_pin {
            Some(ref mut pwr) if !leave_pwr_alone => {
                pwr.try_set_high().ok();
            }
            _ => {}
        }

        Ok(())
    }

    fn power_on(&self) -> Result<(), Error> {
        let vint_value = match self.config.try_borrow_mut()?.vint_pin {
            Some(ref _vint) => false,
            _ => false,
        };

        if vint_value || self.is_alive(1).is_ok() {
            defmt::debug!("powering on, module is already on, flushing config...");
        } else {
            defmt::debug!("powering on.");
            if let Some(ref mut pwr) = self.config.try_borrow_mut()?.pwr_pin {
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
        if self.config.try_borrow()?.baud_rate > 230_400_u32 {
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

        if self.config.try_borrow()?.hex_mode {
            self.send_internal(
                &SetHexMode {
                    hex_mode_disable: HexMode::Enabled,
                },
                false,
            )?;
        } else {
            self.send_internal(
                &SetHexMode {
                    hex_mode_disable: HexMode::Disabled,
                },
                false,
            )?;
        }

        // self.send_internal(&general::IdentificationInformation { n: 9 }, true)?;

        // Stay in airplane mode until commanded to connect
        self.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::AirplaneMode,
                rst: None,
            },
            false,
        )?;

        if self.config.try_borrow()?.flow_control {
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

    pub fn spin(&self) -> nb::Result<(), Error> {
        self.handle_urc().ok();

        let mut fsm = self
            .fsm
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let new_state = match fsm.get_state() {
            State::Init => match self.initialize(true) {
                Ok(()) => State::PowerOn,
                Err(_) => return fsm.retry_or_fail(),
            },
            State::PowerOn => match self.power_on() {
                Ok(()) => State::Configure,
                Err(_) => return fsm.retry_or_fail(),
            },
            State::Configure => match self.configure() {
                Ok(()) => State::SimPin,
                Err(_) => return fsm.retry_or_fail(),
            },
            State::SimPin => {
                self.send_internal(
                    &SetNetworkRegistrationStatus {
                        n: NetworkRegistrationUrcConfig::UrcEnabled,
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

                // TODO: Handle SIM Pin insert here

                // let APNInfo {
                //     apn,
                //     user_name,
                //     password,
                // } = &self.config.apn_info;

                // let apn = match apn {
                //     Apn::Given(apn) => apn.as_str(),
                //     Apn::Automatic => {
                //         // Lookup APN in DB!
                //         unimplemented!()
                //     }
                // };

                // Set up authentication mode, if required
                // if user_name.is_some() || password.is_some() {
                //     // TODO: `AT+UAUTHREQ=` here
                //     {
                //         cid: CONTEXT_ID,
                //         auth_type: AuthType::Automatic,
                //         username: apn_info.user_name.unwrap_or_default(),
                //         password: apn_info.user_name.unwrap_or_default(),
                //     }
                // }

                // Now come out of airplane mode and try to register
                self.send_internal(
                    &SetModuleFunctionality {
                        fun: Functionality::Full,
                        rst: None,
                    },
                    true,
                )?;

                // self.send_internal(
                //     &mobile_control::SetAutomaticTimezoneUpdate {
                //         on_off: AutomaticTimezone::EnabledLocal,
                //     },
                //     true,
                // )?;

                State::SignalQuality
            }
            State::SignalQuality => State::RegisteringNetwork,
            State::RegisteringNetwork => {
                if !self
                    .network_status
                    .try_borrow()
                    .map_err(|e| nb::Error::Other(e.into()))?
                    .is_registered()
                {
                    if !fsm.is_retry() {
                        self.network_registration(None)?;
                    }

                    return fsm.retry_or_fail();
                }
                State::AttachingNetwork
            }
            State::AttachingNetwork => {
                if self.attach().is_err() {
                    return fsm.retry_or_fail();
                }
                State::ActivatingContext
            }
            State::ActivatingContext => {
                // Activate a PDP context
                //TODO: Check AT+CGACT? to verify that `CONTEXT_ID` is active

                // If not active, help it on its way.
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

                if self
                    .send_internal(
                        &SetPacketSwitchedAction {
                            profile_id: PROFILE_ID,
                            action: PacketSwitchedAction::Activate,
                        },
                        true,
                    )
                    .is_err()
                {
                    defmt::warn!("Failed UPSDA!");
                    return fsm.retry_or_fail();
                }

                State::Ready
            }
            State::Ready => {
                self.sockets
                    .try_borrow_mut()
                    .map_err(|e| nb::Error::Other(e.into()))?
                    .iter_mut()
                    .try_for_each(|(_, socket)| self.socket_ingress(socket))?;

                return Ok(());
            }
        };

        fsm.set_state(new_state);

        Err(nb::Error::WouldBlock)
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

    pub fn network_registration(&self, plmn: Option<&str>) -> Result<(), Error> {
        match plmn {
            Some(p) => {
                defmt::debug!("Manual network registration to {:str}", p);
                // TODO: https://github.com/ARMmbed/mbed-os/blob/master/connectivity/cellular/source/framework/AT/AT_CellularNetwork.cpp#L227
                // self.send_internal(
                //     &SetOperatorSelection {
                //         mode: OperatorSelectionMode::Manual,
                //     },
                //     true,
                // )?;
            }
            None => {
                defmt::debug!("Automatic network registration");
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
            }
        }

        Ok(())
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
        if self.fsm.try_borrow()?.get_state() == State::Init {
            return Err(Error::Uninitialized);
        }

        self.send_internal(cmd, true)
    }
}
