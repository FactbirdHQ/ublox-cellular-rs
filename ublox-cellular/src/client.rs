use atat::{clock::Clock, AtatClient};
use embedded_hal::digital::blocking::{InputPin, OutputPin};
use fugit::ExtU32;
use ublox_sockets::SocketSet;

use crate::{
    command::device_lock::{responses::PinStatus, types::PinStatusCode, GetPinStatus},
    command::{
        control::{types::*, *},
        mobile_control::{types::*, *},
        system_features::{types::*, *},
        *,
    },
    command::{
        network_service::{
            responses::OperatorSelection, types::OperatorSelectionMode, GetOperatorSelection,
            SetOperatorSelection,
        },
        psn::{types::PSEventReportingMode, SetPacketSwitchedEventReporting},
    },
    config::Config,
    error::{from_clock, Error, GenericError},
    network::{AtTx, Network},
    power::PowerState,
    registration::ConnectionState,
    services::data::ContextState,
};
use ip_transport_layer::{types::HexMode, SetHexMode};
use network_service::{types::NetworkRegistrationUrcConfig, SetNetworkRegistrationStatus};
use psn::{
    types::{EPSNetworkRegistrationUrcConfig, GPRSNetworkRegistrationUrcConfig},
    SetEPSNetworkRegistrationStatus, SetGPRSNetworkRegistrationStatus,
};
use sms::{types::MessageWaitingMode, SetMessageWaitingIndication};

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

pub struct Device<C, CLK, RST, DTR, PWR, VINT, const TIMER_HZ: u32, const N: usize, const L: usize>
where
    C: AtatClient,
    CLK: 'static + Clock<TIMER_HZ>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
    pub(crate) config: Config<RST, DTR, PWR, VINT>,
    pub(crate) network: Network<C, CLK, TIMER_HZ>,

    pub(crate) state: State,
    pub(crate) power_state: PowerState,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: Option<&'static mut SocketSet<TIMER_HZ, N, L>>,
}

impl<C, CLK, RST, DTR, PWR, VINT, const TIMER_HZ: u32, const N: usize, const L: usize> Drop
    for Device<C, CLK, RST, DTR, PWR, VINT, TIMER_HZ, N, L>
where
    C: AtatClient,
    CLK: Clock<TIMER_HZ>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
    fn drop(&mut self) {
        if self.state != State::Off {
            self.state = State::Off;
            self.hard_power_off().ok();
        }
    }
}

impl<C, CLK, RST, DTR, PWR, VINT, const TIMER_HZ: u32, const N: usize, const L: usize>
    Device<C, CLK, RST, DTR, PWR, VINT, TIMER_HZ, N, L>
where
    C: AtatClient,
    CLK: Clock<TIMER_HZ>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
    /// Create new u-blox device
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use bbqueue::BBBuffer;
    /// use ublox_cellular::prelude::*;
    /// use ublox_cellular::atat;
    /// use ublox_cellular::{Config, GsmClient};
    ///
    /// const RX_BUF_LEN: usize = 256;
    /// const RES_CAPACITY: usize = 256;
    /// const URC_CAPACITY: usize = 256;
    /// const MAX_SOCKET_COUNT: usize = 1;
    /// static mut RES_QUEUE: BBBuffer<RES_CAPACITY> = BBBuffer::new();
    /// static mut URC_QUEUE: BBBuffer<URC_CAPACITY> = BBBuffer::new();
    ///
    /// type UbloxResetPin = gpio::Gpio26<gpio::Output>;
    ///
    /// let queues = atat::Queues {
    ///     res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
    ///     urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
    /// };
    ///
    /// let (atat_client, mut ingress) =
    ///     atat::ClientBuilder::<_, _, _, TIMER_HZ, RX_BUF_LEN, RES_CAPACITY, URC_CAPACITY>::new(
    ///         tx,
    ///         timer::SysTimer::new(),
    ///         atat::AtDigester::<Urc>::new(),
    ///         atat::Config::new(atat::Mode::Timeout),
    ///     )
    ///     .build(queues);
    ///
    ///
    /// let mut modem = GsmClient::<
    ///     _,
    ///     _,
    ///     UbloxResetPin,
    ///     gpio::Gpio0<gpio::Output>,
    ///     gpio::Gpio0<gpio::Output>,
    ///     gpio::Gpio0<gpio::Input>,
    ///     TIMER_HZ,
    ///     MAX_SOCKET_COUNT,
    ///     SOCKET_RING_BUFFER_LEN,
    /// >::new(
    ///     atat_client,
    ///     timer::SysTimer::new(),
    ///     Config::new("").with_flow_control().with_rst(reset),
    /// );
    /// ```
    pub fn new(client: C, timer: CLK, config: Config<RST, DTR, PWR, VINT>) -> Self {
        let mut device = Device {
            config,
            state: State::Off,
            power_state: PowerState::Off,
            network: Network::new(AtTx::new(client, 10), timer),
            sockets: None,
        };

        device.power_state = device.power_state().unwrap_or(PowerState::Off);
        device
    }

    /// Set storage for TCP/UDP sockets
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use ublox_cellular::sockets::SocketSet;
    ///
    /// const TIMER_HZ: u32 = 1000;
    /// const MAX_SOCKET_COUNT: usize = 1;
    /// const SOCKET_RING_BUFFER_LEN: usize = 1024;
    ///
    /// static mut SOCKET_SET: Option<SocketSet<TIMER_HZ, MAX_SOCKET_COUNT, SOCKET_RING_BUFFER_LEN>> = None;
    ///
    /// unsafe {
    ///     SOCKET_SET = Some(SocketSet::new());
    /// }
    ///
    /// modem.set_socket_storage(unsafe { SOCKET_SET.as_mut().unwrap() });
    /// ```
    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<TIMER_HZ, N, L>) {
        socket_set.prune();
        self.sockets.replace(socket_set);
    }

    pub fn take_socket_storage(&mut self) -> Option<&'static mut SocketSet<TIMER_HZ, N, L>> {
        self.sockets.take()
    }

    /// Run modem state machine
    ///
    /// For typical use case only this is needed to manage modem automatically.
    /// It does everything from turning on the modem, configuring it and managing network connection.
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

        if self.config.baud_rate > 230_400_u32 {
            // Needs a way to reconfigure uart baud rate temporarily
            // Relevant issue: https://github.com/rust-embedded/embedded-hal/issues/79
            return Err(Error::_Unknown);

            // self.network.send_internal(
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
        } else {
            // Make sure AT commands parser is in clean state.
            self.network.at_tx.reset()?;
            self.power_on()?;
        }

        // At this point, if is_alive fails, the configured Baud rate is probably wrong
        self.is_alive(20).map_err(|_| Error::BaudDetection)?;

        // Extended errors on
        self.network.send_internal(
            &SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            },
            false,
        )?;

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

        if self.config.hex_mode {
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
        if self.config.flow_control {
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

            self.network
                .status
                .timer
                .start(1.secs())
                .map_err(from_clock)?;
            nb::block!(self.network.status.timer.wait()).map_err(from_clock)?;
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

        return Err(Error::Busy);
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
            self.network.at_tx.reset()?;
            if self.power_on().is_err() {
                self.hard_reset()?;
            }

            self.power_state = PowerState::On;
        } else if matches!(self.state, State::FullyInitialized) {
            return Ok(());
        }

        // At this point, if is_alive fails, the configured Baud rate is probably wrong
        self.is_alive(20).map_err(|_| Error::BaudDetection)?;

        self.setup_at_commands()?;
        self.select_sim_card()?;

        // Disable Message Waiting URCs (UMWI)
        // SARA-R5 does not support it
        #[cfg(not(feature = "sara-r5"))]
        self.network.send_internal(
            &SetMessageWaitingIndication {
                mode: MessageWaitingMode::Disabled,
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
                rst: Some(ResetMode::DontReset),
            },
            true,
        )?;

        // self.network.send_internal(
        //     &SetRadioAccessTechnology {
        //         selected_act: RadioAccessTechnologySelected::GsmUmtsLte(RatPreferred::Lte, RatPreferred::Utran),
        //     },
        //     false,
        // )?;

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
        self.network.at_tx.reset()?;
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            sockets.prune();
        }

        // Allow ATAT some time to clear the buffers
        self.network
            .status
            .timer
            .start(300.millis())
            .map_err(from_clock)?;
        nb::block!(self.network.status.timer.wait()).map_err(from_clock)?;

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

        // CREG URC
        self.network.send_internal(
            &SetNetworkRegistrationStatus {
                n: NetworkRegistrationUrcConfig::UrcVerbose,
            },
            true,
        )?;

        // CGREG URC
        self.network.send_internal(
            &SetGPRSNetworkRegistrationStatus {
                n: GPRSNetworkRegistrationUrcConfig::UrcVerbose,
            },
            true,
        )?;

        // CEREG URC
        self.network.send_internal(
            &SetEPSNetworkRegistrationStatus {
                n: EPSNetworkRegistrationUrcConfig::UrcVerbose,
            },
            true,
        )?;

        Ok(())
    }

    fn handle_urc_internal(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            let ts = self.network.status.timer.now();
            self.network
                .at_tx
                .handle_urc(|urc| {
                    match urc {
                        Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                            info!("[URC] SocketClosed {}", socket.0);
                            if let Some((_, mut sock)) =
                                sockets.iter_mut().find(|(handle, _)| *handle == socket)
                            {
                                sock.closed_by_remote(ts);
                            }
                        }
                        Urc::SocketDataAvailable(
                            ip_transport_layer::urc::SocketDataAvailable { socket, length },
                        )
                        | Urc::SocketDataAvailableUDP(
                            ip_transport_layer::urc::SocketDataAvailable { socket, length },
                        ) => {
                            trace!("[Socket({})] {} bytes available", socket.0, length as u16);
                            if let Some((_, mut sock)) =
                                sockets.iter_mut().find(|(handle, _)| *handle == socket)
                            {
                                sock.set_available_data(length);
                            }
                        }
                        _ => return false,
                    }
                    true
                })
                .map_err(Error::Network)
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

    pub fn handle_urc<F: FnOnce(Urc) -> bool>(&mut self, f: F) -> Result<(), Error> {
        self.network.at_tx.handle_urc(f).map_err(Error::Network)
    }
}

#[cfg(test)]
mod tests {
    use ublox_sockets::{SocketHandle, TcpSocket, UdpSocket};

    use super::*;
    use crate::test_helpers::{MockAtClient, MockTimer};
    use crate::{config::Config, services::data::ContextState, APNInfo};

    const SOCKET_SIZE: usize = 128;
    const SOCKET_SET_LEN: usize = 2;
    const TIMER_HZ: u32 = 1000;

    static mut SOCKET_SET: Option<SocketSet<TIMER_HZ, SOCKET_SET_LEN, SOCKET_SIZE>> = None;

    #[test]
    #[ignore]
    fn prune_on_initialize() {
        let client = MockAtClient::new(0);
        let timer = MockTimer::new(None);
        let config = Config::default();

        let socket_set: &'static mut _ = unsafe {
            SOCKET_SET = Some(SocketSet::new());
            SOCKET_SET.as_mut().unwrap_or_else(|| {
                panic!("Failed to get the static com_queue");
            })
        };

        let mut device = Device::<_, _, _, _, _, _, TIMER_HZ, SOCKET_SET_LEN, SOCKET_SIZE>::new(
            client, timer, config,
        );
        device.set_socket_storage(socket_set);

        // device.fsm.set_state(State::Connected);
        // assert_eq!(device.fsm.get_state(), State::Connected);
        device.state = State::FullyInitialized;
        device.power_state = PowerState::On;
        // assert_eq!(device.spin(), Ok(()));

        device.network.context_state = ContextState::Active;

        let mut data_service = device.data_service(&APNInfo::default()).unwrap();

        if let Some(ref mut sockets) = data_service.sockets {
            sockets
                .add(TcpSocket::new(0))
                .expect("Failed to add new tcp socket!");
            assert_eq!(sockets.len(), 1);

            let mut tcp = sockets
                .get::<TcpSocket<TIMER_HZ, SOCKET_SIZE>>(SocketHandle(0))
                .expect("Failed to get socket");

            assert_eq!(tcp.rx_window(), SOCKET_SIZE);
            let socket_data = b"This is socket data!!";
            tcp.rx_enqueue_slice(socket_data);
            assert_eq!(tcp.recv_queue(), socket_data.len());
            assert_eq!(tcp.rx_window(), SOCKET_SIZE - socket_data.len());

            sockets
                .add(UdpSocket::new(1))
                .expect("Failed to add new udp socket!");
            assert_eq!(sockets.len(), 2);

            assert!(sockets.add(UdpSocket::new(0)).is_err());
        } else {
            panic!()
        }

        drop(data_service);

        device.clear_buffers().expect("Failed to clear buffers");

        let mut data_service = device.data_service(&APNInfo::default()).unwrap();
        if let Some(ref mut sockets) = data_service.sockets {
            assert_eq!(sockets.len(), 0);

            sockets
                .add(TcpSocket::new(0))
                .expect("Failed to add new tcp socket!");
            assert_eq!(sockets.len(), 1);

            let tcp = sockets
                .get::<TcpSocket<TIMER_HZ, SOCKET_SIZE>>(SocketHandle(0))
                .expect("Failed to get socket");
            assert_eq!(tcp.recv_queue(), 0);
        } else {
            panic!()
        }
    }
}
