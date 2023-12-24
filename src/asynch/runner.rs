use core::str::FromStr;

use crate::{command::Urc, config::CellularConfig};

use super::state::{self, LinkState};
use crate::asynch::state::OperationState;
use crate::asynch::state::OperationState::{PowerDown, PowerUp};
use crate::command::control::types::{Circuit108Behaviour, Circuit109Behaviour, FlowControl};
use crate::command::control::{SetCircuit108Behaviour, SetCircuit109Behaviour, SetFlowControl};
use crate::command::device_lock::responses::PinStatus;
use crate::command::device_lock::types::PinStatusCode;
use crate::command::device_lock::GetPinStatus;
use crate::command::general::{GetCCID, GetFirmwareVersion, GetModelId, IdentificationInformation};
use crate::command::gpio::types::{GpioInPull, GpioMode, GpioOutValue};
use crate::command::gpio::SetGpioConfiguration;
use crate::command::ip_transport_layer::types::HexMode;
use crate::command::ip_transport_layer::SetHexMode;
use crate::command::mobile_control::types::{Functionality, ResetMode, TerminationErrorMode};
use crate::command::mobile_control::{SetModuleFunctionality, SetReportMobileTerminationError};
use crate::command::system_features::types::PowerSavingMode;
use crate::command::system_features::SetPowerSavingControl;
use crate::command::AT;
use crate::error::Error;
use crate::error::GenericError::Timeout;
use crate::module_timing::{boot_time, reset_time};
use atat::{asynch::AtatClient, UrcSubscription};
use embassy_futures::select::select;
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::{InputPin, OutputPin};
use heapless::String;
use no_std_net::{Ipv4Addr, Ipv6Addr};

use embassy_futures::select::Either;

use super::AtHandle;

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'d, AT: AtatClient, C: CellularConfig, const URC_CAPACITY: usize> {
    ch: state::Runner<'d>,
    at: AtHandle<'d, AT>,
    config: C,
    urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
}

impl<'d, AT: AtatClient, C: CellularConfig, const URC_CAPACITY: usize>
    Runner<'d, AT, C, URC_CAPACITY>
{
    pub(crate) fn new(
        ch: state::Runner<'d>,
        at: AtHandle<'d, AT>,
        config: C,
        urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
    ) -> Self {
        Self {
            ch,
            at,
            config,
            urc_subscription,
        }
    }

    // TODO: crate visibility only makes sense if reset and co are also crate visibility
    // pub(crate) async fn init(&mut self) -> Result<(), Error> {
    pub async fn init(&mut self) -> Result<(), Error> {
        // Initilize a new ublox device to a known state (set RS232 settings)
        debug!("Initializing module");
        // Hard reset module
        if Ok(false) == self.has_power().await {
            self.power_up().await?;
        };
        self.reset().await?;
        self.is_alive().await?;

        Ok(())
    }

    pub async fn is_alive(&mut self) -> Result<bool, Error> {
        let has_power = self.has_power().await?;
        if !has_power {
            return Err(Error::PoweredDown);
        }

        let alive = match self.at.send(AT).await {
            Ok(_) => {
                return Ok(true);
            }
            Err(err) => {
                return Err(Error::Atat(err));
            }
        };
    }

    pub async fn has_power(&mut self) -> Result<bool, Error> {
        if let Some(pin) = self.config.vint_pin() {
            if pin.is_high().map_err(|_| Error::IoPin)? {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            info!("No VInt pin configured");
            Ok(true)
        }
    }

    pub async fn power_up(&mut self) -> Result<(), Error> {
        if !self.has_power().await? {
            if let Some(pin) = self.config.power_pin() {
                pin.set_low().map_err(|_| Error::IoPin)?;
                Timer::after(crate::module_timing::pwr_on_time()).await;
                pin.set_high().map_err(|_| Error::IoPin)?;
                Timer::after(boot_time()).await;
                self.ch.set_power_state(OperationState::PowerUp);
                debug!("Powered up");
                Ok(())
            } else {
                warn!("No power pin configured");
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub async fn power_down(&mut self) -> Result<(), Error> {
        if self.has_power().await? {
            if let Some(pin) = self.config.power_pin() {
                pin.set_low().map_err(|_| Error::IoPin)?;
                Timer::after(crate::module_timing::pwr_off_time()).await;
                pin.set_high().map_err(|_| Error::IoPin)?;
                self.ch.set_power_state(OperationState::PowerDown);
                debug!("Powered down");
                Ok(())
            } else {
                warn!("No power pin configured");
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub async fn init_at(&mut self) -> Result<(), Error> {
        if !self.is_alive().await? {
            return Err(Error::PoweredDown);
        }

        // Extended errors on
        self.at
            .send(SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            })
            .await?;

        // Select SIM
        self.at
            .send(SetGpioConfiguration {
                gpio_id: 25,
                gpio_mode: GpioMode::Output(GpioOutValue::High),
            })
            .await?;

        #[cfg(any(feature = "lara-r6"))]
        self.at
            .send(SetGpioConfiguration {
                gpio_id: 42,
                gpio_mode: GpioMode::Input(GpioInPull::NoPull),
            })
            .await?;

        let model_id = self.at.send(GetModelId).await?;

        // self.at.send(
        //     &IdentificationInformation {
        //         n: 9
        //     },
        // ).await?;

        self.at.send(GetFirmwareVersion).await?;

        self.select_sim_card().await?;

        let ccid = self.at.send(GetCCID).await?;
        info!("CCID: {}", ccid.ccid);
        // DCD circuit (109) changes in accordance with the carrier
        self.at
            .send(SetCircuit109Behaviour {
                value: Circuit109Behaviour::ChangesWithCarrier,
            })
            .await?;

        // Ignore changes to DTR
        self.at
            .send(SetCircuit108Behaviour {
                value: Circuit108Behaviour::Ignore,
            })
            .await?;

        // Switch off UART power saving until it is integrated into this API
        self.at
            .send(SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
            })
            .await?;

        if C::HEX_MODE {
            self.at
                .send(SetHexMode {
                    hex_mode_disable: HexMode::Enabled,
                })
                .await?;
        } else {
            self.at
                .send(SetHexMode {
                    hex_mode_disable: HexMode::Disabled,
                })
                .await?;
        }

        // Tell module whether we support flow control
        // FIXME: Use AT+IFC=2,2 instead of AT&K here
        if C::FLOW_CONTROL {
            self.at
                .send(SetFlowControl {
                    value: FlowControl::RtsCts,
                })
                .await?;
        } else {
            self.at
                .send(SetFlowControl {
                    value: FlowControl::Disabled,
                })
                .await?;
        }
        Ok(())
    }

    pub async fn select_sim_card(&mut self) -> Result<(), Error> {
        for _ in 0..2 {
            match self.at.send(GetPinStatus).await {
                Ok(PinStatus { code }) if code == PinStatusCode::Ready => {
                    return Ok(());
                }
                _ => {}
            }

            Timer::after(Duration::from_secs(1)).await;
        }

        // There was an error initializing the SIM
        // We've seen issues on uBlox-based devices, as a precation, we'll cycle
        // the modem here through minimal/full functional state.
        self.at
            .send(SetModuleFunctionality {
                fun: Functionality::Minimum,
                // SARA-R5 This parameter can be used only when <fun> is 1, 4 or 19
                #[cfg(feature = "sara-r5")]
                rst: None,
                #[cfg(not(feature = "sara-r5"))]
                rst: Some(ResetMode::DontReset),
            })
            .await?;
        self.at
            .send(SetModuleFunctionality {
                fun: Functionality::Full,
                rst: Some(ResetMode::DontReset),
            })
            .await?;

        Err(Error::Busy)
    }

    pub async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Cellular Module");
        if let Some(pin) = self.config.reset_pin() {
            pin.set_low().ok();
            Timer::after(reset_time()).await;
            pin.set_high().ok();
            Timer::after(boot_time()).await;
            self.is_alive().await?;
        } else {
            warn!("No reset pin configured");
        }
        Ok(())
    }

    pub async fn restart(&mut self, store: bool) -> Result<(), Error> {
        Ok(())
    }

    pub async fn run(mut self) -> ! {
        match self.has_power().await.ok() {
            Some(false) => {
                self.ch.set_power_state(OperationState::PowerDown);
            }
            Some(true) => {
                self.ch.set_power_state(OperationState::PowerUp);
            }
            None => {
                self.ch.set_power_state(OperationState::PowerDown);
            }
        }
        loop {
            match select(
                self.ch.state_runner().wait_for_desired_state_change(),
                self.urc_subscription.next_message_pure(),
            )
            .await
            {
                Either::First(desired_state) => {
                    info!("Desired state: {:?}", desired_state);
                    if let Err(err) = desired_state {
                        error!("Error in desired_state retrival: {:?}", err);
                        continue;
                    }
                    let desired_state = desired_state.unwrap();
                    if 0 >= desired_state as isize - self.ch.state_runner().power_state() as isize {
                        debug!(
                            "Power steps was negative, power down: {}",
                            desired_state as isize - self.ch.state_runner().power_state() as isize
                        );
                        self.power_down().await.ok();
                        self.ch.set_power_state(OperationState::PowerDown);
                    }
                    let start_state = self.ch.state_runner().power_state() as isize;
                    let steps = desired_state as isize - start_state;
                    for step in 0..=steps {
                        debug!(
                            "State transition {} steps: {} -> {}, {}",
                            steps,
                            start_state,
                            start_state + step,
                            step
                        );
                        let next_state = start_state + step;
                        match OperationState::try_from(next_state) {
                            Ok(OperationState::PowerDown) => {}
                            Ok(OperationState::PowerUp) => match self.power_up().await {
                                Ok(_) => {
                                    self.ch.set_power_state(OperationState::PowerUp);
                                }
                                Err(err) => {
                                    error!("Error in power_up: {:?}", err);
                                    break;
                                }
                            },
                            Ok(OperationState::Alive) => match self.is_alive().await {
                                Ok(_) => {
                                    debug!("Will set Alive");
                                    self.ch.set_power_state(OperationState::Alive);
                                    debug!("Set Alive");
                                }
                                Err(err) => {
                                    error!("Error in is_alive: {:?}", err);
                                    break;
                                }
                            },
                            Ok(OperationState::Initialized) => match self.init_at().await {
                                Ok(_) => {
                                    self.ch.set_power_state(OperationState::Initialized);
                                }
                                Err(err) => {
                                    error!("Error in init_at: {:?}", err);
                                    break;
                                }
                            },
                            Ok(OperationState::Connected) => {
                                todo!()
                            }
                            Ok(OperationState::DataEstablished) => {
                                todo!()
                            }
                            Err(_) => {
                                error!("State transition next_state not valid: start_state={}, next_state={}, steps={} ", start_state, next_state, steps);
                                break;
                            }
                        }
                    }
                }
                Either::Second(event) => {
                    self.handle_urc(event).await;
                }
            }
        }
    }

    async fn handle_urc(&mut self, event: Urc) -> Error {
        match event {
            // Handle network URCs
            Urc::NetworkDetach => todo!(),
            Urc::MobileStationDetach => todo!(),
            Urc::NetworkDeactivate => todo!(),
            Urc::MobileStationDeactivate => todo!(),
            Urc::NetworkPDNDeactivate => todo!(),
            Urc::MobileStationPDNDeactivate => todo!(),
            Urc::SocketDataAvailable(_) => todo!(),
            Urc::SocketDataAvailableUDP(_) => todo!(),
            Urc::DataConnectionActivated(_) => todo!(),
            Urc::DataConnectionDeactivated(_) => todo!(),
            Urc::SocketClosed(_) => todo!(),
            Urc::MessageWaitingIndication(_) => todo!(),
            Urc::ExtendedPSNetworkRegistration(_) => todo!(),
            Urc::HttpResponse(_) => todo!(),
        };
    }
}
