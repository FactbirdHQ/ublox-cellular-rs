use atat::AtatClient;
use core::convert::TryInto;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_time::{duration::*, Clock};
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
        error::UbloxError,
        network_service::{
            responses::OperatorSelection, types::OperatorSelectionMode, GetOperatorSelection,
            SetOperatorSelection,
        },
        psn::{types::PSEventReportingMode, SetPacketSwitchedEventReporting}, general::GetFirmwareVersion,
    },
    config::Config,
    error::{Error, GenericError},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum State {
    Off,
    On,
}

pub struct Device<C, CLK, RST, DTR, PWR, VINT, const N: usize, const L: usize>
where
    C: AtatClient,
    CLK: 'static + Clock,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
    pub(crate) config: Config<RST, DTR, PWR, VINT>,
    pub(crate) network: Network<C, CLK>,

    pub(crate) state: State,
    pub(crate) power_state: PowerState,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: Option<&'static mut SocketSet<CLK, N, L>>,
}

// impl<C, CLK, RST, DTR, PWR, VINT, const N: usize, const L: usize> Drop
//     for Device<C, CLK, RST, DTR, PWR, VINT, N, L>
// where
//     C: AtatClient,
//     CLK: Clock,
//     RST: OutputPin,
//     PWR: OutputPin,
//     DTR: OutputPin,
//     VINT: InputPin,
// {
//     fn drop(&mut self) {
//         if self.state != State::Off {
//             self.state = State::Off;
//             self.hard_power_off().ok();
//         }
//     }
// }

impl<C, CLK, RST, DTR, PWR, VINT, const N: usize, const L: usize>
    Device<C, CLK, RST, DTR, PWR, VINT, N, L>
where
    C: AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
{
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

        Err(Error::Busy)
    }

    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<CLK, N, L>) {
        socket_set.prune();
        self.sockets.replace(socket_set);
    }

    pub fn take_socket_storage(&mut self) -> Option<&'static mut SocketSet<CLK, N, L>> {
        self.sockets.take()
    }

    pub fn initialize(&mut self) -> Result<(), Error>
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if !matches!(self.power_state, PowerState::On) {
            // Always re-configure the module when power has been off
            self.state = State::Off;

            // Catch states where we have no vint sense, and the module is already in powered mode,
            // but for some reason doesn't answer to AT commands.
            // This usually happens on programming after modem power on.
            if self.power_on().is_err() {
                self.hard_reset()?;
            }

            self.power_state = PowerState::On;
        } else if matches!(self.state, State::On) {
            return Ok(());
        }

        // At this point, if is_alive fails, the configured Baud rate is probably wrong
        self.is_alive(20).map_err(|_| Error::BaudDetection)?;

        self.configure()?;

        // let firmware = self.network.send_internal(
        //     &general::IdentificationInformation {
        //         n: 9
        //     },
        //     false,
        // )?;

        // defmt::info!("Current firmware version: {}", firmware);

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
            .new_timer(300_u32.milliseconds())
            .start()?
            .wait()?;

        Ok(())
    }

    pub(crate) fn configure(&mut self) -> Result<(), Error> {
        // Always re-configure the PDP contexts if we reconfigure the module
        self.network.context_state = ContextState::Setup;

        self.clear_buffers()?;

        let current_device_baud_rate = self.network.send_internal(&GetDataRate, false)?;
        defmt::info!(
            "Device is configured for baud rate: {}",
            current_device_baud_rate.data_rate
        );

        // Check if we need to reconfigure the baud rate
        if self.config.baud_rate != current_device_baud_rate.data_rate {
            defmt::info!(
                "Reconfiguring ublox baud rate to: {}",
                self.config.baud_rate
            );
            let new_rate = self
                .config
                .baud_rate
                .try_into()
                .map_err(|_| Error::_Unknown)?;
            self.network
                .send_internal(&SetDataRate { rate: new_rate }, false)?;

            // NOTE: On the UART AT interface, after the reception of the "OK" result code for the +IPR command, the DTE
            // shall wait for at least 100 ms before issuing a new AT command; this is to guarantee a proper baud rate
            // reconfiguration.
            self.network
                .status
                .timer
                .new_timer(200_u32.milliseconds())
                .start()?
                .wait()?;

            // After reconfigurating the baud rate we need to tell the caller that they to reconnect to the device
            // with a new Baud rate.
            return Err(Error::NeedsBaudReconnect(new_rate));
        } else {
            // TODO: This should only be done the first time we switch baud rate

            defmt::info!("Baudrate seems to be correct. Persisting current settings.");
            self.network
                .send_internal(&StoreCurrentConfig { profile: 0 }, false)?;

            self.network.send_internal(&ModuleSwitchOff, false)?;

            self.network
                .status
                .timer
                .new_timer(1_u32.seconds())
                .start()?
                .wait()?;

            self.power_on()?;
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

        self.state = State::On;
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

    fn handle_urc_internal(&mut self) -> Result<(), Error>
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if let Some(ref mut sockets) = self.sockets.as_deref_mut() {
            let ts = self
                .network
                .status
                .timer
                .try_now()
                .map_err(|e| Error::Generic(GenericError::Time(e.into())))?;
            self.network
                .at_tx
                .handle_urc(|urc| {
                    match urc {
                        Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                            defmt::info!("[URC] SocketClosed {=u8}", socket.0);
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
                            defmt::trace!(
                                "[Socket({=u8})] {=u16} bytes available",
                                socket.0,
                                length as u16
                            );
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

    pub(crate) fn process_events(&mut self) -> Result<(), Error>
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if self.power_state != PowerState::On {
            return Err(Error::Uninitialized);
        }

        self.handle_urc_internal()?;

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
            if self.network.context_state == ContextState::Active {
                self.network.context_state = ContextState::Activating;
            }
            Err(nb::Error::WouldBlock)
        }
    }

    pub fn send_at<A, const LEN: usize>(&mut self, cmd: &A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
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

    static mut SOCKET_SET: Option<SocketSet<MockTimer, SOCKET_SET_LEN, SOCKET_SIZE>> = None;

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
            Device::<_, _, _, _, _, _, SOCKET_SET_LEN, SOCKET_SIZE>::new(client, timer, config);
        device.set_socket_storage(socket_set);

        // device.fsm.set_state(State::Connected);
        // assert_eq!(device.fsm.get_state(), State::Connected);
        device.state = State::On;
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
                .get::<TcpSocket<_, SOCKET_SIZE>>(SocketHandle(0))
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
                .get::<TcpSocket<_, SOCKET_SIZE>>(SocketHandle(0))
                .expect("Failed to get socket");
            assert_eq!(tcp.recv_queue(), 0);
        } else {
            panic!()
        }
    }
}
