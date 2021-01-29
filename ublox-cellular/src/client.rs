use atat::AtatClient;
use core::{cell::RefCell, convert::TryInto};
use embedded_hal::{
    blocking::delay::DelayMs,
    digital::{InputPin, OutputPin},
    timer::CountDown,
};
use heapless::{ArrayLength, Bucket, Pos};

use crate::{
    command::device_lock::GetPinStatus,
    command::device_lock::{responses::PinStatus, types::PinStatusCode},
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
        psn::{types::GPRSAttachedState, SetGPRSAttached},
    },
    config::{Config, NoPin},
    error::Error,
    network::{AtTx, Network},
    services::data::socket::{Socket, SocketSet},
    state::Event,
    state::StateMachine,
    State,
};
use ip_transport_layer::{types::HexMode, SetHexMode};
use network_service::{types::NetworkRegistrationUrcConfig, SetNetworkRegistrationStatus};
use psn::{
    types::{EPSNetworkRegistrationUrcConfig, GPRSNetworkRegistrationUrcConfig},
    SetEPSNetworkRegistrationStatus, SetGPRSNetworkRegistrationStatus,
};
use sms::{types::MessageWaitingMode, SetMessageWaitingIndication};

pub struct Device<C, DLY, N, L, RST = NoPin, DTR = NoPin, PWR = NoPin, VINT = NoPin>
where
    C: AtatClient,
    DLY: DelayMs<u32> + CountDown,
    N: 'static
        + ArrayLength<Option<Socket<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    pub(crate) fsm: StateMachine,
    pub(crate) config: Config<RST, DTR, PWR, VINT>,
    pub(crate) delay: DLY,
    pub(crate) network: Network<C>,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: Option<RefCell<&'static mut SocketSet<N, L>>>,
}

impl<C, DLY, N, L, RST, DTR, PWR, VINT> Device<C, DLY, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    DLY: DelayMs<u32> + CountDown,
    DLY::Time: From<u32>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<Socket<L>>> + ArrayLength<Bucket<u8, usize>> + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn new(client: C, delay: DLY, config: Config<RST, DTR, PWR, VINT>) -> Self {
        Device {
            fsm: StateMachine::new(),
            config,
            delay,
            network: Network::new(AtTx::new(client, 5)),
            sockets: None,
        }
    }

    pub fn factory_reset(&mut self) -> Result<(), Error> {
        self.network.send_internal(
            &SetFactoryConfiguration {
                fs_op: FSFactoryRestoreType::AllFiles,
                nvm_op: NVMFactoryRestoreType::NVMFlashSectors,
            },
            true,
        )?;

        defmt::info!("Succefully factory reset modem! ");
        self.network.push_event(Event::FactoryReset)?;

        Ok(())
    }

    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<N, L>) {
        self.sockets = Some(RefCell::new(socket_set));
    }

    pub(crate) fn initialize(&mut self, leave_pwr_alone: bool) -> Result<(), Error> {
        defmt::info!(
            "Initialising with PWR_ON pin: {:bool} and VInt pin: {:bool}. Using PWR_ON pin: {:bool}",
            self.config.pwr_pin.is_some(),
            self.config.vint_pin.is_some(),
            !leave_pwr_alone
        );

        let vint_value = match self.config.vint_pin {
            Some(ref _vint) => false,
            _ => false,
        };

        if vint_value || self.is_alive(3).is_ok() {
            defmt::debug!("powering on, module is already on, flushing config...");
        } else {
            defmt::debug!("powering on.");
            match self.config.pwr_pin {
                Some(ref mut pwr) if !leave_pwr_alone => {
                    pwr.try_set_high().ok();
                    self.delay
                        .try_delay_ms(crate::module_cfg::constants::PWR_ON_PULL_TIME_MS)
                        .map_err(|_| Error::Busy)?;
                    pwr.try_set_low().ok();
                    self.delay
                        .try_delay_ms(crate::module_cfg::constants::PWR_ON_PULL_TIME_MS)
                        .map_err(|_| Error::Busy)?;
                    pwr.try_set_high().ok();
                }
                _ => {
                    // Software restart
                    self.restart(false)?;
                }
            }
        }

        self.delay
            .try_delay_ms(crate::module_cfg::constants::BOOT_WAIT_TIME_MS)
            .map_err(|_| Error::Busy)?;
        self.is_alive(10)?;

        self.clear_buffers()?;

        self.configure()?;

        self.network.push_event(Event::PwrOn)?;

        Ok(())
    }

    pub(crate) fn clear_buffers(&mut self) -> Result<(), Error> {
        self.network.at_tx.clear_urc_queue()?;
        self.network.clear_events()?;
        if let Some(ref sockets) = self.sockets {
            sockets.try_borrow_mut()?.prune();
        }
        Ok(())
    }

    /// Check that the cellular module is alive.
    ///
    /// See if the cellular module is responding at the AT interface by poking
    /// it with "AT" up to `attempts` times, waiting 1 second for an "OK"
    /// response each time
    pub(crate) fn is_alive(&self, attempts: u8) -> Result<(), Error> {
        let mut error = Error::BaudDetection;
        for _ in 0..attempts {
            match self.network.send_internal(&AT, false) {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => error = e.into(),
            };
        }
        Err(error)
    }

    pub(crate) fn configure(&mut self) -> Result<(), Error> {
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

        // Extended errors on
        self.network.send_internal(
            &SetReportMobileTerminationError {
                n: TerminationErrorMode::Disabled,
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

        // self.network.send_internal(&general::IdentificationInformation { n: 9 }, true)?;

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

        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            },
            true,
        )?;

        // Disable Message Waiting URCs (UMWI)
        self.network.send_internal(
            &SetMessageWaitingIndication {
                mode: MessageWaitingMode::Disabled,
            },
            false,
        )?;

        match self.network.send_internal(&GetPinStatus, true) {
            Ok(PinStatus { code }) if code != PinStatusCode::Ready => {
                // FIXME: Handle SIM Pin here
                defmt::error!("PIN status not ready!!");
                return Err(Error::Busy);
            }
            Err(e) => {
                // Short-circuit to restart with sim-reset on error
                self.fsm.set_max_retry_attempts(0);
                return Err(e.into());
            }
            _ => {}
        }

        let OperatorSelection { mode, .. } =
            self.network.send_internal(&GetOperatorSelection, true)?;

        if mode != OperatorSelectionMode::Automatic {
            self.network.send_internal(
                &SetOperatorSelection {
                    mode: OperatorSelectionMode::Automatic,
                },
                true,
            )?;
        }

        self.enable_registration_urcs()?;

        Ok(())
    }

    #[inline]
    pub(crate) fn restart(&self, sim_reset: bool) -> Result<(), Error> {
        let fun = if sim_reset {
            Functionality::SilentResetWithSimReset
        } else {
            Functionality::SilentReset
        };

        self.network
            .send_internal(&SetModuleFunctionality { fun, rst: None }, false)?;

        self.network.push_event(Event::PwrOff)?;
        Ok(())
    }

    pub(crate) fn enable_registration_urcs(&self) -> Result<(), Error> {
        // if packet domain event reporting is not set it's not a stopper. We
        // might lack some events when we are dropped from the network.
        // TODO: Re-enable this when it works, and is useful!
        if self
            .network
            .set_packet_domain_event_reporting(false)
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

    pub fn attach(&self) -> Result<(), Error> {
        self.network.send_internal(
            &SetGPRSAttached {
                state: GPRSAttachedState::Attached,
            },
            true,
        )?;

        Ok(())
    }

    fn handle_urc(&self, state: State) -> Result<(), Error> {
        if let Some(ref sockets) = self.sockets {
            self.network
                .at_tx
                .handle_urc(|urc| {
                    match urc {
                        // Blindly swallow all URC's if state is unknown
                        _ if matches!(state, State::Unknown) => {}
                        Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                            defmt::info!("[URC] SocketClosed {:u8}", socket.0);
                            if let Ok(mut sockets) = sockets.try_borrow_mut() {
                                sockets.remove(socket).ok();
                            }
                        }
                        Urc::SocketDataAvailable(
                            ip_transport_layer::urc::SocketDataAvailable { socket, length },
                        )
                        | Urc::SocketDataAvailableUDP(
                            ip_transport_layer::urc::SocketDataAvailable { socket, length },
                        ) => {
                            defmt::trace!(
                                "[Socket({:u8})] {:u16} bytes available",
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
                                    "[Socket({:u8})] Failed to borrow socketset!",
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

    pub(crate) fn handle_events(&mut self) -> nb::Result<(), Error> {
        let state = self.fsm.get_state();
        self.handle_urc(state).map_err(Error::from)?;
        self.network.handle_urc().map_err(Error::from)?;

        // Always let events propagate the state of the FSM.
        while let Some(event) = self.network.get_event().map_err(Error::from)? {
            if let Ok(cell_event) = event.try_into() {
                self.fsm.handle_event(cell_event);
            }
        }

        Ok(())
    }

    pub fn spin(&mut self) -> nb::Result<bool, Error> {
        self.handle_events()?;

        if self.fsm.is_retry() {
            if let Err(nb::Error::WouldBlock) = self.delay.try_wait() {
                return Err(nb::Error::WouldBlock);
            }
        }

        let state = self.fsm.get_state();
        let res = match state {
            State::Unknown => self.restart(true),
            State::Off => self.initialize(true),
            _ => Ok(()),
        };

        if res.is_err() {
            match self.fsm.retry_or_fail(&mut self.delay) {
                nb::Error::WouldBlock => return Err(nb::Error::WouldBlock),
                nb::Error::Other(_) => {
                    if self.network.clear_events().is_err() {
                        defmt::error!("Failed to clear events after failed state transition!");
                    }
                    self.fsm.set_state(State::Unknown);
                }
            }
        }

        match state {
            State::On | State::Registered => Ok(false),
            State::Connected => Ok(true),
            _ => Err(nb::Error::WouldBlock),
        }
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        // At any point after init state, we should be able to fully send AT
        // commands.
        if matches!(self.fsm.get_state(), State::Unknown | State::Off) {
            return Err(Error::Uninitialized);
        }

        Ok(self.network.send_internal(cmd, true)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        services::data::ContextState,
        sockets::{SocketHandle, TcpSocket, UdpSocket},
        APNInfo,
    };
    use crate::{
        test_helpers::{MockAtClient, MockTimer},
        ContextId,
    };
    use atat::typenum::Unsigned;
    use heapless::consts;

    type SocketSize = consts::U128;
    type SocketSetLen = consts::U2;

    static mut SOCKET_SET: Option<SocketSet<SocketSetLen, SocketSize>> = None;

    #[test]
    fn prune_on_initialize() {
        let client = MockAtClient::new();
        let timer = MockTimer::new();
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

        device.fsm.set_state(State::Connected);
        assert_eq!(device.fsm.get_state(), State::Connected);
        assert_eq!(device.spin(), Ok(true));

        device.network.context_state.set(ContextState::Active);

        let data_service = device
            .data_service(ContextId(0), &APNInfo::default())
            .unwrap();

        let mut sockets = data_service.sockets.borrow_mut();

        sockets
            .add(TcpSocket::new(0))
            .expect("Failed to add new tcp socket!");
        assert_eq!(sockets.len(), 1);

        let mut tcp = sockets
            .get::<TcpSocket<_>>(SocketHandle(0))
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

        let data_service = device
            .data_service(ContextId(0), &APNInfo::default())
            .unwrap();

        let mut sockets = data_service.sockets.borrow_mut();
        assert_eq!(sockets.len(), 0);

        sockets
            .add(TcpSocket::new(0))
            .expect("Failed to add new tcp socket!");
        assert_eq!(sockets.len(), 1);

        let tcp = sockets
            .get::<TcpSocket<_>>(SocketHandle(0))
            .expect("Failed to get socket");

        assert_eq!(tcp.recv_queue(), 0);
    }
}
