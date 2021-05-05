use atat::AtatClient;
use core::{cell::RefCell, convert::TryInto};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_time::{duration::*, Clock};
use heapless::{ArrayLength, Bucket, Pos};

use crate::{command::device_lock::{responses::PinStatus, types::PinStatusCode, GetPinStatus}, command::{
        control::{types::*, *},
        mobile_control::{types::*, *},
        system_features::{types::*, *},
        *,
    }, command::{error::UbloxError, network_service::{
            responses::OperatorSelection, types::OperatorSelectionMode, GetOperatorSelection,
            SetOperatorSelection,
        }, psn::{types::PSEventReportingMode, SetPacketSwitchedEventReporting}}, config::Config, error::{Error, GenericError}, network::{AtTx, Network}, power::PowerState, registration::ConnectionState, services::data::{
        socket::{Socket, SocketSet},
        ContextState,
    }};
use ip_transport_layer::{types::HexMode, SetHexMode};
use network_service::{
    types::{NetworkRegistrationUrcConfig, RadioAccessTechnologySelected, RatPreferred},
    SetNetworkRegistrationStatus, SetRadioAccessTechnology,
};
use psn::{
    types::{EPSNetworkRegistrationUrcConfig, GPRSNetworkRegistrationUrcConfig},
    SetEPSNetworkRegistrationStatus, SetGPRSNetworkRegistrationStatus,
};
use sms::{types::MessageWaitingMode, SetMessageWaitingIndication};

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum State {
    Off,
    On,
}

pub struct Device<C, CLK, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    CLK: 'static + Clock,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: 'static
        + ArrayLength<Option<Socket<L, CLK>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    pub(crate) config: Config<RST, DTR, PWR, VINT>,
    pub(crate) network: Network<C, CLK>,

    pub(crate) state: State,
    pub(crate) power_state: PowerState,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: Option<RefCell<&'static mut SocketSet<N, L, CLK>>>,
}

// TODO:
impl<C, CLK, N, L, RST, DTR, PWR, VINT> Drop for Device<C, CLK, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    CLK: Clock,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<Socket<L, CLK>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    fn drop(&mut self) {
        if self.state != State::Off {
            self.state = State::Off;
            self.hard_power_off().ok();
        }
    }
}

impl<C, CLK, N, L, RST, DTR, PWR, VINT> Device<C, CLK, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    CLK: Clock,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<Socket<L, CLK>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn new(client: C, timer: CLK, config: Config<RST, DTR, PWR, VINT>) -> Self {
        let mut device = Device {
            config,
            state: State::Off,
            power_state: PowerState::Off,
            network: Network::new(AtTx::new(client, 10), timer),
            sockets: None,
        };

        let power_state = device.power_state().unwrap_or(PowerState::Off);
        device.power_state = power_state;
        device
    }

    pub fn select_sim_card(&mut self) -> Result<(), Error> {
        for _ in 0..2 {
            match self.network.send_internal(&GetPinStatus, true) {
                Ok(PinStatus { code }) if code == PinStatusCode::Ready => {
                    return Ok(());
                }
                _ => {}
            }

            self.network
                .status
                .try_borrow()?
                .timer
                .new_timer(1.seconds())
                .start()?
                .wait()?;
        }

        // There was an error initializing the SIM
        // We've seen issues on uBlox-based devices, as a precation, we'll cycle
        // the modem here through minimal/full functional state.
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Minimum,
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

    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<N, L, CLK>) {
        self.sockets = Some(RefCell::new(socket_set));
    }

    pub fn initialize(&mut self) -> Result<(), Error>
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if self.power_state != PowerState::On {
            // Always re-configure the module when power has been off
            self.state = State::Off;

            // Catch states where we have no vint sense, and the module is already in powered mode,
            // but for some reason doesn't answer to AT commands.
            // This usually happens on programming after modem power on.
            if self.power_on().is_err() {
                self.hard_reset()?;
            }

            self.is_alive(10)?;

            self.power_state = PowerState::On;
        }

        self.configure()?;

        Ok(())
    }

    pub(crate) fn clear_buffers(&mut self) -> Result<(), Error> {
        self.network.at_tx.reset()?;
        if let Some(ref sockets) = self.sockets {
            sockets.try_borrow_mut()?.prune();
        }

        // Allow ATAT some time to clear the buffers
        self.network
            .status
            .try_borrow()?
            .timer
            .new_timer(300_u32.milliseconds())
            .start()?
            .wait()?;

        Ok(())
    }

    pub(crate) fn configure(&mut self) -> Result<(), Error> {
        if matches!(self.state, State::On) {
            return Ok(());
        }

        // Always re-configure the PDP contexts if we reconfigure the module
        self.network.context_state.set(ContextState::Setup);

        self.is_alive(2)?;

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
        }

        self.select_sim_card()?;

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

        // Disable Message Waiting URCs (UMWI)
        self.network.send_internal(
            &SetMessageWaitingIndication {
                mode: MessageWaitingMode::Disabled,
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

        let mut ns = self.network.status.try_borrow_mut()?;
        ns.reset();
        ns.set_connection_state(ConnectionState::Connecting);
        drop(ns);

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

        self.state = State::On;
        Ok(())
    }

    pub(crate) fn enable_registration_urcs(&self) -> Result<(), Error> {
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
            defmt::warn!("Packet domain event reporting set failed");
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

    fn handle_urc(&self) -> Result<(), Error> {
        if let Some(ref sockets) = self.sockets {
            self.network
                .at_tx
                .handle_urc(|urc| {
                    match urc {
                        Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                            defmt::info!("[URC] SocketClosed {=u8}", socket.0);
                            if let Ok(mut sockets) = sockets.try_borrow_mut() {
                                if sockets.remove(socket).is_err() {
                                    defmt::warn!("Socket already closed!");
                                }
                            }
                        }
                        Urc::SocketDataAvailable(
                            ip_transport_layer::urc::SocketDataAvailable { socket, length },
                        )
                        | Urc::SocketDataAvailableUDP(
                            ip_transport_layer::urc::SocketDataAvailable { socket, length },
                        ) => {
                            defmt::trace!(
                                "[Socket({=u8})] {=u16} bytes available",
                                socket.0,
                                length as u16
                            );
                            if let Ok(mut sockets) = sockets.try_borrow_mut() {
                                if let Some((_, mut sock)) =
                                    sockets.iter_mut().find(|(handle, _)| *handle == socket)
                                {
                                    sock.set_available_data(length);
                                }
                            } else {
                                defmt::warn!(
                                    "[Socket({=u8})] Failed to borrow socketset!",
                                    socket.0
                                );
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

    pub(crate) fn process_events(&mut self) -> Result<(), Error>
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if self.power_state != PowerState::On {
            return Err(Error::Uninitialized);
        }

        self.handle_urc()?;

        match self.network.process_events() {
            // Catch "Resetting the modem due to the network registration timeout"
            // as well as consecutive AT timeouts and do a hard reset.
            Err(crate::network::Error::Generic(GenericError::Timeout)) => self.hard_reset(),
            result => result.map_err(Error::from),
        }
    }

    pub fn spin(&mut self) -> nb::Result<(), Error>
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        let res = self.initialize();

        self.process_events().map_err(Error::from)?;

        res?;

        if self.network.is_connected().map_err(Error::from)? && self.state == State::On {
            Ok(())
        } else {
            // Reset context state if data connection is lost (This will act as a safeguard if a URC is missed)
            if self.network.context_state.get() == ContextState::Active {
                self.network.context_state.set(ContextState::Activating);
            }
            Err(nb::Error::WouldBlock)
        }
    }

    pub fn send_at<A>(&self, cmd: &A) -> Result<A::Response, Error> 
    where 
        A: atat::AtatCmd,
        A::Error: Into<UbloxError>,
    {
        // At any point after init state, we should be able to fully send AT
        // commands.
        if self.state != State::On {
            defmt::error!("Still not initialized!");
            return Err(Error::Uninitialized);
        }

        Ok(self.network.send_internal(cmd, true)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{MockAtClient, MockTimer};
    use crate::{
        config::Config,
        services::data::ContextState,
        sockets::{SocketHandle, TcpSocket, UdpSocket},
        APNInfo,
    };
    use atat::typenum::Unsigned;
    use heapless::consts;

    type SocketSize = consts::U128;
    type SocketSetLen = consts::U2;

    static mut SOCKET_SET: Option<SocketSet<SocketSetLen, SocketSize, MockTimer>> = None;

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

        let mut device =
            Device::<_, _, SocketSetLen, SocketSize, _, _, _, _>::new(client, timer, config);
        device.set_socket_storage(socket_set);

        // device.fsm.set_state(State::Connected);
        // assert_eq!(device.fsm.get_state(), State::Connected);
        device.state = State::On;
        device.power_state = PowerState::On;
        // assert_eq!(device.spin(), Ok(()));

        device.network.context_state.set(ContextState::Active);

        let data_service = device.data_service(&APNInfo::default()).unwrap();

        let mut sockets = data_service.sockets.borrow_mut();

        sockets
            .add(TcpSocket::new(0))
            .expect("Failed to add new tcp socket!");
        assert_eq!(sockets.len(), 1);

        let mut tcp = sockets
            .get::<TcpSocket<_, _>>(SocketHandle(0))
            .expect("Failed to get socket");

        assert_eq!(tcp.rx_window(), SocketSize::to_usize());
        let socket_data = b"This is socket data!!";
        tcp.rx_enqueue_slice(socket_data);
        assert_eq!(tcp.recv_queue(), socket_data.len());
        assert_eq!(tcp.rx_window(), SocketSize::to_usize() - socket_data.len());

        sockets
            .add(UdpSocket::new(1))
            .expect("Failed to add new udp socket!");
        assert_eq!(sockets.len(), 2);

        assert!(sockets.add(UdpSocket::new(0)).is_err());

        drop(sockets);
        drop(data_service);

        device.clear_buffers().expect("Failed to clear buffers");

        let data_service = device.data_service(&APNInfo::default()).unwrap();

        let mut sockets = data_service.sockets.borrow_mut();
        assert_eq!(sockets.len(), 0);

        sockets
            .add(TcpSocket::new(0))
            .expect("Failed to add new tcp socket!");
        assert_eq!(sockets.len(), 1);

        let tcp = sockets
            .get::<TcpSocket<_, _>>(SocketHandle(0))
            .expect("Failed to get socket");

        assert_eq!(tcp.recv_queue(), 0);
    }
}
