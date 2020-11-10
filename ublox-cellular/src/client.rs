use atat::AtatClient;
use core::cell::RefCell;
use embedded_hal::{
    blocking::delay::DelayMs,
    digital::{InputPin, OutputPin},
    timer::CountDown,
};
use heapless::{ArrayLength, Bucket, Pos};

use crate::{
    command::{
        control::{types::*, *},
        mobile_control::{types::*, *},
        system_features::{types::*, *},
        *,
    },
    config::{Config, NoPin},
    error::Error,
    network::{AtTx, Network},
    services::data::socket::{SocketSet, SocketSetItem},
    state::StateMachine,
    State,
};
use general::{responses::CCID, GetCCID};
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
        + ArrayLength<Option<SocketSetItem<L>>>
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
    N: ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn new(client: C, delay: DLY, config: Config<RST, DTR, PWR, VINT>) -> Self {
        Device {
            fsm: StateMachine::new(),
            config,
            delay,
            network: Network::new(AtTx::new(client, 20)),
            sockets: None,
        }
    }

    pub fn set_socket_storage(&mut self, socket_set: &'static mut SocketSet<N, L>) {
        self.sockets = Some(RefCell::new(socket_set));
    }

    pub(crate) fn initialize(&mut self, leave_pwr_alone: bool) -> Result<(), Error> {
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

        Ok(())
    }

    pub(crate) fn power_on(&mut self) -> Result<(), Error> {
        let vint_value = match self.config.vint_pin {
            Some(ref _vint) => false,
            _ => false,
        };

        if vint_value || self.is_alive(1).is_ok() {
            defmt::debug!("powering on, module is already on, flushing config...");
        } else {
            defmt::debug!("powering on.");
            if let Some(ref mut pwr) = self.config.pwr_pin {
                pwr.try_set_low().ok();
                self.delay
                    .try_delay_ms(crate::module_cfg::constants::PWR_ON_PULL_TIME_MS)
                    .map_err(|_| Error::Busy)?;
                pwr.try_set_high().ok();
            } else {
                // Software restart
                self.restart()?;
            }
            self.delay
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

    pub(crate) fn configure(&self) -> Result<(), Error> {
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

        // TODO: switch off UART power saving until it is integrated into this API
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

        // Stay in airplane mode until commanded to register
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::AirplaneMode,
                rst: None,
            },
            false,
        )?;

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
            &SetMessageWaitingIndication {
                mode: MessageWaitingMode::Disabled,
            },
            false,
        )?;

        Ok(())
    }

    #[inline]
    pub(crate) fn restart(&self) -> Result<(), Error> {
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::SilentReset,
                rst: None,
            },
            false,
        )?;
        Ok(())
    }

    pub(crate) fn enable_network_urcs(&self) -> Result<(), Error> {
        self.network.send_internal(
            &SetNetworkRegistrationStatus {
                n: NetworkRegistrationUrcConfig::UrcEnabled,
            },
            true,
        )?;

        self.network.send_internal(
            &SetGPRSNetworkRegistrationStatus {
                n: GPRSNetworkRegistrationUrcConfig::UrcEnabled,
            },
            true,
        )?;

        self.network.send_internal(
            &SetEPSNetworkRegistrationStatus {
                n: EPSNetworkRegistrationUrcConfig::UrcEnabled,
            },
            true,
        )?;
        Ok(())
    }

    pub fn spin(&mut self) -> nb::Result<(), Error> {
        self.network.handle_urc().ok();

        if self.fsm.is_retry() {
            if let Err(nb::Error::WouldBlock) = self.delay.try_wait() {
                return Err(nb::Error::WouldBlock);
            }
        }

        let new_state = match self.fsm.get_state() {
            State::Init => match self.initialize(true) {
                Ok(()) => Ok(State::PowerOn),
                Err(_) => Err(State::Init),
            },
            State::PowerOn => match self.power_on() {
                Ok(()) => Ok(State::Configure),
                Err(_) => Err(State::PowerOn),
            },
            State::Configure => match self.configure() {
                Ok(()) => Ok(State::SimPin),
                Err(_) => Err(State::PowerOn),
            },
            State::SimPin => {
                self.enable_network_urcs()?;
                // TODO: Handle SIM Pin here

                self.network.send_internal(
                    &psn::SetPDPContextDefinition {
                        cid: crate::ContextId(1),
                        pdp_type: "IP",
                        apn: "em",
                    },
                    true,
                ).map_err(|e| nb::Error::Other(e.into()))?;

                // Now come out of airplane mode and try to register
                self.network
                    .send_internal(
                        &SetModuleFunctionality {
                            fun: Functionality::Full,
                            rst: None,
                        },
                        true,
                    )
                    .map_err(|e| nb::Error::Other(e.into()))?;

                // FIXME:
                // self.network.send_internal(
                //     &mobile_control::SetAutomaticTimezoneUpdate {
                //         on_off: AutomaticTimezone::EnabledLocal,
                //     },
                //     true,
                // )?;

                Ok(State::SignalQuality)
            }
            State::SignalQuality => {
                let CCID { ccid } = self
                    .network
                    .send_internal(&GetCCID, true)
                    .map_err(|e| nb::Error::Other(e.into()))?;

                defmt::info!("CCID: {:?}", ccid.to_le_bytes());

                Ok(State::RegisteringNetwork)
            }
            State::RegisteringNetwork => match self.network.register(None) {
                Ok(_) => Ok(State::AttachingNetwork),
                Err(_) => Err(State::PowerOn),
            },
            State::AttachingNetwork => match self.network.attach() {
                Ok(_) => Ok(State::Connected),
                Err(_) => Err(State::PowerOn),
            },
            State::Connected => match self
                .network
                .is_registered()
                .map_err(|e| nb::Error::Other(e.into()))?
            {
                true => {
                    // Reset the retry attempts on connected, as this
                    // essentially is a success path.
                    self.fsm.reset();
                    return Ok(());
                }
                false => {
                    // If registration status changed from "Registered", check
                    // up to 3 times to make sure, and back to registering if
                    // it's still disconnected.
                    self.fsm.set_max_retry_attempts(3);
                    Err(State::RegisteringNetwork)
                }
            },
        };

        match new_state {
            Ok(new_state) => self.fsm.set_state(new_state),
            Err(err_state) => {
                if let nb::Error::Other(Error::StateTimeout) =
                    self.fsm.retry_or_fail(&mut self.delay)
                {
                    self.fsm.set_state(err_state);
                }
            }
        }

        Err(nb::Error::WouldBlock)
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        if self.fsm.get_state() == State::Init {
            return Err(Error::Uninitialized);
        }

        Ok(self.network.send_internal(cmd, true)?)
    }
}
