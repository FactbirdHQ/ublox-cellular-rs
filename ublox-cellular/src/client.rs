use atat::{blocking::AtatClient, AtatUrcChannel, UrcSubscription};
use embassy_time::Duration;
use ublox_sockets::SocketSet;

use crate::{
    blocking_timer::BlockingTimer,
    command::device_lock::{responses::PinStatus, types::PinStatusCode, GetPinStatus},
    command::{
        control::{
            types::{Circuit108Behaviour, Circuit109Behaviour, FlowControl},
            SetCircuit108Behaviour, SetCircuit109Behaviour, SetFlowControl,
        },
        ip_transport_layer,
        mobile_control::{
            types::{AutomaticTimezone, Functionality, ResetMode, TerminationErrorMode},
            SetAutomaticTimezoneUpdate, SetModuleFunctionality, SetReportMobileTerminationError,
        },
        network_service, psn,
        system_features::{types::PowerSavingMode, SetPowerSavingControl},
        Urc,
    },
    command::{
        general::{GetCCID, GetFirmwareVersion, GetModelId},
        gpio::{
            types::{GpioInPull, GpioMode, GpioOutValue},
            SetGpioConfiguration,
        },
        network_service::{
            responses::{OperatorSelection, SignalQuality},
            types::OperatorSelectionMode,
            GetOperatorSelection, GetSignalQuality, SetOperatorSelection,
        },
        psn::{types::PSEventReportingMode, SetPacketSwitchedEventReporting},
    },
    config::CellularConfig,
    error::{Error, GenericError},
    network::{AtTx, Network},
    power::PowerState,
    registration::ConnectionState,
    services::data::ContextState,
    UbloxCellularBuffers, UbloxCellularIngress, UbloxCellularUrcChannel,
};
use ip_transport_layer::{types::HexMode, SetHexMode};
use network_service::{types::NetworkRegistrationUrcConfig, SetNetworkRegistrationStatus};
use psn::{
    types::{EPSNetworkRegistrationUrcConfig, GPRSNetworkRegistrationUrcConfig},
    SetEPSNetworkRegistrationStatus, SetGPRSNetworkRegistrationStatus,
};

pub(crate) const URC_CAPACITY: usize = 3;
pub(crate) const URC_SUBSCRIBERS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    /// Device is off
    Off,
    /// Device is able to respond to AT commands
    AtInitialized,
    /// Device is fully initialized
    FullyInitialized,
}

pub struct Device<'buf, 'sub, AtCl, AtUrcCh, Config, const N: usize, const L: usize> {
    pub(crate) config: Config,
    pub(crate) network: Network<'sub, AtCl>,
    urc_channel: &'buf AtUrcCh,
    urc_subscription: UrcSubscription<'sub, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,

    pub(crate) state: State,
    pub(crate) power_state: PowerState,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: Option<&'static mut SocketSet<N, L>>,
}

impl<'buf, 'sub, W, Config, const INGRESS_BUF_SIZE: usize, const N: usize, const L: usize>
    Device<
        'buf,
        'sub,
        atat::blocking::Client<'buf, W, INGRESS_BUF_SIZE>,
        UbloxCellularUrcChannel,
        Config,
        N,
        L,
    >
where
    'buf: 'sub,
    W: embedded_io::Write,
    Config: CellularConfig,
{
    /// Create new u-blox device
    ///
    /// Look for [`data_service`](Device::data_service) how to handle data connection automatically.
    ///
    pub fn from_buffers(
        buffers: &'buf UbloxCellularBuffers<INGRESS_BUF_SIZE>,
        tx: W,
        config: Config,
    ) -> (UbloxCellularIngress<INGRESS_BUF_SIZE>, Self) {
        let (ingress, client) = buffers.split_blocking(
            tx,
            atat::DefaultDigester::<Urc>::default(),
            atat::Config::default(),
        );

        (ingress, Device::new(client, &buffers.urc_channel, config))
    }
}

impl<'buf, 'sub, AtCl, AtUrcCh, Config, const N: usize, const L: usize>
    Device<'buf, 'sub, AtCl, AtUrcCh, Config, N, L>
where
    'buf: 'sub,
    AtCl: AtatClient,
    AtUrcCh: AtatUrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    Config: CellularConfig,
{
    pub fn new(client: AtCl, urc_channel: &'buf AtUrcCh, config: Config) -> Self {
        let network_urc_subscription = urc_channel.subscribe().unwrap();
        Self {
            config,
            network: Network::new(AtTx::new(client, network_urc_subscription)),
            state: State::Off,
            power_state: PowerState::Off,
            sockets: None,
            urc_channel,
            urc_subscription: urc_channel.subscribe().unwrap(),
        }
    }
}

impl<'buf, 'sub, AtCl, AtUrcCh, Config, const N: usize, const L: usize>
    Device<'buf, 'sub, AtCl, AtUrcCh, Config, N, L>
where
    'buf: 'sub,
    AtCl: AtatClient,
    Config: CellularConfig,
{
    /// Set storage for TCP/UDP sockets
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ublox_cellular::sockets::SocketSet;
    ///
    /// const MAX_SOCKET_COUNT: usize = 1;
    /// const SOCKET_RING_BUFFER_LEN: usize = 1024;
    ///
    /// static mut SOCKET_SET: Option<SocketSet<MAX_SOCKET_COUNT, SOCKET_RING_BUFFER_LEN>> = None;
    ///
    /// unsafe {
    ///     SOCKET_SET = Some(SocketSet::new());
    /// }
    ///
    /// modem.set_socket_storage(unsafe { SOCKET_SET.as_mut().unwrap() });
    /// ```
    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<N, L>) {
        socket_set.prune();
        self.sockets.replace(socket_set);
    }

    pub fn take_socket_storage(&mut self) -> Option<&'static mut SocketSet<N, L>> {
        self.sockets.take()
    }

    pub fn signal_strength(&mut self) -> Result<SignalQuality, Error> {
        self.send_at(&GetSignalQuality)
    }
    /// Run modem state machine
    ///
    /// Turns on modem if needed and processes URCs.
    /// Typically it is not needed to use it directly. However it can be useful for manually handling network connections.
    /// For fully automatic data connection handling use [`data_service`](Device::data_service).
    ///
    /// This must be called periodically in a loop.
    pub fn spin(&mut self) -> nb::Result<(), Error> {
        let res = self.initialize();

        self.process_events().map_err(Error::from)?;

        res?;

        if self.network.is_connected().map_err(Error::from)?
            && self.state == State::FullyInitialized
        {
            Ok(())
        } else {
            // Reset context state if data connection is lost (This will act as a safeguard if a URC is missed)
            if self.network.context_state == ContextState::Active {
                self.network.context_state = ContextState::Activating;
            }
            Err(nb::Error::WouldBlock)
        }
    }

    /// Setup only essential settings to use AT commands
    ///
    /// Nornally this is not used and AT interface is setup in [`initialize`](Device::initialize).
    /// However it is useful if you want to send some AT commands before modem is fully initialized.
    /// After this [`send_at`](Device::send_at) can be used.
    pub fn setup_at_commands(&mut self) -> Result<(), Error> {
        // Always re-configure the PDP contexts if we reconfigure the module
        self.network.context_state = ContextState::Setup;

        self.clear_buffers()?;

        self.power_on()?;

        // At this point, if is_alive fails, the configured Baud rate is probably wrong
        if let Err(e) = self.is_alive(5).map_err(|_| Error::BaudDetection) {
            if self.hard_reset().is_err() {
                self.hard_power_off()?;
                BlockingTimer::after(Duration::from_secs(1)).wait();
            }
            return Err(e);
        }

        // Extended errors on
        self.network.send_internal(
            &SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            },
            false,
        )?;

        // Select SIM
        self.network.send_internal(
            &SetGpioConfiguration {
                gpio_id: 25,
                gpio_mode: GpioMode::Output(GpioOutValue::High),
            },
            false,
        )?;

        #[cfg(any(feature = "lara-r6"))]
        self.network.send_internal(
            &SetGpioConfiguration {
                gpio_id: 42,
                gpio_mode: GpioMode::Input(GpioInPull::NoPull),
            },
            false,
        )?;

        self.network.send_internal(&GetModelId, false)?;

        // self.network.send_internal(
        //     &IdentificationInformation {
        //         n: 9
        //     },
        //     false,
        // )?;

        self.network.send_internal(&GetFirmwareVersion, false)?;

        self.select_sim_card()?;

        self.network.send_internal(&GetCCID, false)?;

        // DCD circuit (109) changes in accordance with the carrier
        self.network.send_internal(
            &SetCircuit109Behaviour {
                value: Circuit109Behaviour::ChangesWithCarrier,
            },
            false,
        )?;

        // Ignore changes to DTR
        self.network.send_internal(
            &SetCircuit108Behaviour {
                value: Circuit108Behaviour::Ignore,
            },
            false,
        )?;

        // Switch off UART power saving until it is integrated into this API
        self.network.send_internal(
            &SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
            },
            false,
        )?;

        if Config::HEX_MODE {
            self.network.send_internal(
                &SetHexMode {
                    hex_mode_disable: HexMode::Enabled,
                },
                false,
            )?;
        } else {
            self.network.send_internal(
                &SetHexMode {
                    hex_mode_disable: HexMode::Disabled,
                },
                false,
            )?;
        }

        // Tell module whether we support flow control
        // FIXME: Use AT+IFC=2,2 instead of AT&K here
        if Config::FLOW_CONTROL {
            self.network.send_internal(
                &SetFlowControl {
                    value: FlowControl::RtsCts,
                },
                false,
            )?;
        } else {
            self.network.send_internal(
                &SetFlowControl {
                    value: FlowControl::Disabled,
                },
                false,
            )?;
        }

        self.state = State::AtInitialized;
        Ok(())
    }

    /// Send AT commands and wait responses from modem
    ///
    /// Modem must be initialized before this works.
    /// For example use [`setup_at_commands`](Device::setup_at_commands) to only initialize AT commands support and nothing else.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ublox_cellular::command::device_data_security::{types::SecurityDataType, RetrieveSecurityMd5};
    ///
    /// modem.setup_at_commands()?;
    /// modem.send_at(&RetrieveSecurityMd5 {
    ///     data_type: SecurityDataType::TrustedRootCA,
    ///     internal_name: "ca_cert",
    /// })?;
    /// info!("md5: {:}", resp.md5_string);
    /// ```
    pub fn send_at<A, const LEN: usize>(&mut self, cmd: &A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
    {
        match self.state {
            State::Off => {
                error!("Device not initialized!");
                return Err(Error::Uninitialized);
            }
            State::AtInitialized | State::FullyInitialized => {}
        }

        Ok(self.network.send_internal(cmd, true)?)
    }

    fn select_sim_card(&mut self) -> Result<(), Error> {
        for _ in 0..2 {
            match self.network.send_internal(&GetPinStatus, true) {
                Ok(PinStatus { code }) if code == PinStatusCode::Ready => {
                    return Ok(());
                }
                _ => {}
            }

            BlockingTimer::after(Duration::from_secs(1)).wait();
        }

        // There was an error initializing the SIM
        // We've seen issues on uBlox-based devices, as a precation, we'll cycle
        // the modem here through minimal/full functional state.
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Minimum,
                // SARA-R5 This parameter can be used only when <fun> is 1, 4 or 19
                #[cfg(feature = "sara-r5")]
                rst: None,
                #[cfg(not(feature = "sara-r5"))]
                rst: Some(ResetMode::DontReset),
            },
            true,
        )?;
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Full,
                rst: Some(ResetMode::DontReset),
            },
            true,
        )?;

        Err(Error::Busy)
    }

    /// Initialize modem fully
    ///
    /// Turns modem on if it is off, configures it and starts registering to network.
    ///
    /// In typical situations user should not use it directly.
    /// This should be used only if you would like to handle connections manually.
    ///
    /// For typical use case use [`spin`][Device::spin] which handles everything automatically.
    pub fn initialize(&mut self) -> Result<(), Error> {
        if self.power_state != PowerState::On {
            // Always re-configure the module when power has been off
            self.state = State::Off;

            // Catch states where we have no vint sense, and the module is already in powered mode,
            // but for some reason doesn't answer to AT commands.
            // This usually happens on programming after modem power on.
            if self.power_on().is_err() {
                self.hard_reset()?;
            }

            self.power_state = PowerState::On;
        } else if matches!(self.state, State::FullyInitialized) {
            return Ok(());
        }

        self.setup_at_commands()?;
        self.select_sim_card()?;

        // Disable Message Waiting URCs (UMWI)
        #[cfg(any(feature = "toby-r2"))]
        self.network.send_internal(
            &crate::command::sms::SetMessageWaitingIndication {
                mode: crate::command::sms::types::MessageWaitingMode::Disabled,
            },
            false,
        )?;

        self.network.send_internal(
            &SetAutomaticTimezoneUpdate {
                on_off: AutomaticTimezone::EnabledLocal,
            },
            false,
        )?;

        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            },
            true,
        )?;

        self.network.status.reset();
        self.network
            .status
            .set_connection_state(ConnectionState::Connecting);

        self.enable_registration_urcs()?;

        // Set automatic operator selection, if not already set
        let OperatorSelection { mode, .. } =
            self.network.send_internal(&GetOperatorSelection, true)?;

        // Only run AT+COPS=0 if currently de-registered, to avoid PLMN reselection
        if !matches!(
            mode,
            OperatorSelectionMode::Automatic | OperatorSelectionMode::Manual
        ) {
            self.network.send_internal(
                &SetOperatorSelection {
                    mode: OperatorSelectionMode::Automatic,
                    format: Some(2),
                },
                true,
            )?;
        }

        self.network.update_registration()?;

        self.network.reset_reg_time()?;

        self.state = State::FullyInitialized;
        Ok(())
    }

    pub(crate) fn clear_buffers(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            sockets.prune();
        }

        Ok(())
    }

    pub(crate) fn enable_registration_urcs(&mut self) -> Result<(), Error> {
        // if packet domain event reporting is not set it's not a stopper. We
        // might lack some events when we are dropped from the network.
        // TODO: Re-enable this when it works, and is useful!
        if self
            .network
            .send_internal(
                &SetPacketSwitchedEventReporting {
                    mode: PSEventReportingMode::CircularBufferUrcs,
                    bfr: None,
                },
                true,
            )
            .is_err()
        {
            warn!("Packet domain event reporting set failed");
        }

        // FIXME: Currently `atat` is unable to distinguish `xREG` family of
        // commands from URC's

        // CREG URC
        self.network.send_internal(
            &SetNetworkRegistrationStatus {
                n: NetworkRegistrationUrcConfig::UrcDisabled,
            },
            true,
        )?;

        // CGREG URC
        self.network.send_internal(
            &SetGPRSNetworkRegistrationStatus {
                n: GPRSNetworkRegistrationUrcConfig::UrcDisabled,
            },
            true,
        )?;

        // CEREG URC
        self.network.send_internal(
            &SetEPSNetworkRegistrationStatus {
                n: EPSNetworkRegistrationUrcConfig::UrcDisabled,
            },
            true,
        )?;

        Ok(())
    }

    fn handle_urc_internal(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            if let Some(urc) = self.urc_subscription.try_next_message_pure() {
                match urc {
                    Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                        info!("[URC] SocketClosed {}", socket.0);
                        if let Some((_, mut sock)) =
                            sockets.iter_mut().find(|(handle, _)| *handle == socket)
                        {
                            sock.closed_by_remote();
                        }
                    }
                    Urc::SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable {
                        socket,
                        length,
                    })
                    | Urc::SocketDataAvailableUDP(ip_transport_layer::urc::SocketDataAvailable {
                        socket,
                        length,
                    }) => {
                        trace!("[Socket({})] {} bytes available", socket.0, length as u16);
                        if let Some((_, mut sock)) =
                            sockets.iter_mut().find(|(handle, _)| *handle == socket)
                        {
                            sock.set_available_data(length);
                        }
                    }
                    _ => {}
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    pub(crate) fn process_events(&mut self) -> Result<(), Error> {
        if self.power_state != PowerState::On {
            return Err(Error::Uninitialized);
        }

        self.handle_urc_internal()?;

        match self.network.process_events() {
            // Catch "Resetting the modem due to the network registration timeout"
            // as well as consecutive AT timeouts and do a hard reset.
            Err(crate::network::Error::Generic(GenericError::Timeout)) => {
                self.hard_reset()?;
                Err(Error::Generic(GenericError::Timeout))
            }
            result => result.map_err(Error::from),
        }
    }
}
