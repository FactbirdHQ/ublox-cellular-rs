use crate::command::control::types::Echo;
use crate::command::control::SetEcho;
use crate::command::psn::GetGPRSAttached;
use crate::command::psn::GetPDPContextState;
use crate::command::psn::SetPDPContextState;

use crate::{command::Urc, config::CellularConfig};

use super::state;
use crate::asynch::state::OperationState;
use crate::command::control::types::{Circuit108Behaviour, Circuit109Behaviour, FlowControl};
use crate::command::control::{SetCircuit108Behaviour, SetCircuit109Behaviour, SetFlowControl};
use crate::command::device_lock::responses::PinStatus;
use crate::command::device_lock::types::PinStatusCode;
use crate::command::device_lock::GetPinStatus;
use crate::command::general::{GetCCID, GetFirmwareVersion, GetModelId};
use crate::command::gpio::types::{GpioInPull, GpioMode, GpioOutValue};
use crate::command::gpio::SetGpioConfiguration;
use crate::command::mobile_control::types::{Functionality, ResetMode, TerminationErrorMode};
use crate::command::mobile_control::{SetModuleFunctionality, SetReportMobileTerminationError};
use crate::command::psn::responses::GPRSAttached;
use crate::command::psn::types::GPRSAttachedState;
use crate::command::psn::types::PDPContextStatus;
use crate::command::system_features::types::PowerSavingMode;
use crate::command::system_features::SetPowerSavingControl;
use crate::command::AT;
use crate::error::Error;
use crate::module_timing::{boot_time, reset_time};
use atat::{asynch::AtatClient, UrcSubscription};
use embassy_futures::select::select;
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::{InputPin, OutputPin};

use crate::command::psn::types::{ContextId, ProfileId};
use embassy_futures::select::Either;

use super::AtHandle;

#[cfg(feature = "ppp")]
pub(crate) const URC_SUBSCRIBERS: usize = 2;

#[cfg(feature = "internal-network-stack")]
pub(crate) const URC_SUBSCRIBERS: usize = 2;

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'d, AT: AtatClient, C: CellularConfig<'d>, const URC_CAPACITY: usize> {
    ch: state::Runner<'d>,
    at: AtHandle<'d, AT>,
    config: C,
    urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
}

impl<'d, AT: AtatClient, C: CellularConfig<'d>, const URC_CAPACITY: usize>
    Runner<'d, AT, C, URC_CAPACITY>
{
    pub(crate) fn new(
        ch: state::Runner<'d>,
        at: AtHandle<'d, AT>,
        config: C,
        urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
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

        Ok(())
    }

    pub async fn is_alive(&mut self) -> Result<bool, Error> {
        if !self.has_power().await? {
            return Err(Error::PoweredDown);
        }

        match self.at.send(&AT).await {
            Ok(_) => Ok(true),
            Err(err) => Err(Error::Atat(err)),
        }
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

    pub async fn wait_for_desired_state(
        &mut self,
        ps: OperationState,
    ) -> Result<OperationState, Error> {
        self.ch.state_runner().wait_for_desired_state(ps).await
    }

    pub async fn power_down(&mut self) -> Result<(), Error> {
        if self.has_power().await? {
            if let Some(pin) = self.config.power_pin() {
                pin.set_low().map_err(|_| Error::IoPin)?;
                Timer::after(crate::module_timing::pwr_off_time()).await;
                pin.set_high().map_err(|_| Error::IoPin)?;
                self.ch.set_power_state(OperationState::PowerDown);
                debug!("Powered down");

                // FIXME: Is this needed?
                Timer::after(Duration::from_millis(1000)).await;

                Ok(())
            } else {
                warn!("No power pin configured");
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// Initializes the network only valid after `init_at`.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the internal network operations fail.
    ///
    pub async fn init_network(&mut self) -> Result<(), Error> {
        // Disable Message Waiting URCs (UMWI)
        #[cfg(any(feature = "toby-r2"))]
        self.at
            .send(&crate::command::sms::SetMessageWaitingIndication {
                mode: crate::command::sms::types::MessageWaitingMode::Disabled,
            })
            .await?;

        self.at
            .send(
                &crate::command::mobile_control::SetAutomaticTimezoneUpdate {
                    on_off: crate::command::mobile_control::types::AutomaticTimezone::EnabledLocal,
                },
            )
            .await?;

        self.at
            .send(&crate::command::mobile_control::SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            })
            .await?;

        self.enable_registration_urcs().await?;

        // Set automatic operator selection, if not already set
        let crate::command::network_service::responses::OperatorSelection { mode, .. } = self
            .at
            .send(&crate::command::network_service::GetOperatorSelection)
            .await?;

        // Only run AT+COPS=0 if currently de-registered, to avoid PLMN reselection
        if !matches!(
            mode,
            crate::command::network_service::types::OperatorSelectionMode::Automatic
                | crate::command::network_service::types::OperatorSelectionMode::Manual
        ) {
            self.at
                .send(&crate::command::network_service::SetOperatorSelection {
                    mode: crate::command::network_service::types::OperatorSelectionMode::Automatic,
                    format: Some(C::OPERATOR_FORMAT as u8),
                })
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn enable_registration_urcs(&mut self) -> Result<(), Error> {
        // if packet domain event reporting is not set it's not a stopper. We
        // might lack some events when we are dropped from the network.
        // TODO: Re-enable this when it works, and is useful!
        if self
            .at
            .send(&crate::command::psn::SetPacketSwitchedEventReporting {
                mode: crate::command::psn::types::PSEventReportingMode::CircularBufferUrcs,
                bfr: None,
            })
            .await
            .is_err()
        {
            warn!("Packet domain event reporting set failed");
        }

        // FIXME: Currently `atat` is unable to distinguish `xREG` family of
        // commands from URC's

        // CREG URC
        self.at.send(
            &crate::command::network_service::SetNetworkRegistrationStatus {
                n: crate::command::network_service::types::NetworkRegistrationUrcConfig::UrcDisabled,
            }).await?;

        // CGREG URC
        self.at
            .send(&crate::command::psn::SetGPRSNetworkRegistrationStatus {
                n: crate::command::psn::types::GPRSNetworkRegistrationUrcConfig::UrcDisabled,
            })
            .await?;

        // CEREG URC
        self.at
            .send(&crate::command::psn::SetEPSNetworkRegistrationStatus {
                n: crate::command::psn::types::EPSNetworkRegistrationUrcConfig::UrcDisabled,
            })
            .await?;

        Ok(())
    }

    /// Reset the module by driving it's `RESET_N` pin low for 50 ms
    ///
    /// **NOTE** This function will reset NVM settings!
    pub async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Cellular Module");
        if let Some(pin) = self.config.reset_pin() {
            pin.set_low().ok();
            Timer::after(reset_time()).await;
            pin.set_high().ok();
            Timer::after(boot_time()).await;
            // self.is_alive().await?;
        } else {
            warn!("No reset pin configured");
        }
        Ok(())
    }

    /// Perform at full factory reset of the module, clearing all NVM sectors in the process
    pub async fn factory_reset(&mut self) -> Result<(), Error> {
        self.at
            .send(&crate::command::system_features::SetFactoryConfiguration {
                fs_op: crate::command::system_features::types::FSFactoryRestoreType::AllFiles,
                nvm_op:
                    crate::command::system_features::types::NVMFactoryRestoreType::NVMFlashSectors,
            })
            .await?;

        info!("Successfully factory reset modem!");

        if self.soft_reset(true).await.is_err() {
            self.reset().await?;
        }

        Ok(())
    }

    /// Reset the module by sending AT CFUN command
    pub async fn soft_reset(&mut self, sim_reset: bool) -> Result<(), Error> {
        trace!(
            "Attempting to soft reset of the modem with sim reset: {}.",
            sim_reset
        );

        let fun = if sim_reset {
            Functionality::SilentResetWithSimReset
        } else {
            Functionality::SilentReset
        };

        match self
            .at
            .send(&SetModuleFunctionality {
                fun,
                // SARA-R5 This parameter can be used only when <fun> is 1, 4 or 19
                #[cfg(feature = "sara-r5")]
                rst: None,
                #[cfg(not(feature = "sara-r5"))]
                rst: Some(ResetMode::DontReset),
            })
            .await
        {
            Ok(_) => {
                info!("Successfully soft reset modem!");
                Ok(())
            }
            Err(err) => {
                error!("Failed to soft reset modem: {:?}", err);
                Err(Error::Atat(err))
            }
        }
    }

    // checks alive status continuiously until it is alive
    async fn check_is_alive_loop(&mut self) -> bool {
        loop {
            if let Ok(alive) = self.is_alive().await {
                return alive;
            }
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    async fn is_network_attached_loop(&mut self) -> bool {
        loop {
            if let Ok(true) = self.is_network_attached().await {
                return true;
            }
            Timer::after(Duration::from_secs(1)).await;
        }
    }

    pub async fn run(&mut self) -> ! {
        match self.has_power().await.ok() {
            Some(true) => {
                self.ch.set_power_state(OperationState::PowerUp);
            }
            Some(false) | None => {
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
                    let _ = self.change_state_to_desired_state(desired_state).await;
                }
                Either::Second(event) => {
                    self.handle_urc(event).await;
                }
            }
        }
    }

    pub async fn change_state_to_desired_state(
        &mut self,
        desired_state: OperationState,
    ) -> Result<(), Error> {
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
                        return Err(err);
                    }
                },
                Ok(OperationState::Initialized) => {
                    #[cfg(not(feature = "ppp"))]
                    match init_at(&mut self.at, C::FLOW_CONTROL).await {
                        Ok(_) => {
                            self.ch.set_power_state(OperationState::Initialized);
                        }
                        Err(err) => {
                            error!("Error in init_at: {:?}", err);
                            return Err(err);
                        }
                    }

                    #[cfg(feature = "ppp")]
                    {
                        self.ch.set_power_state(OperationState::Initialized);
                    }
                }
                Ok(OperationState::Connected) => match self.init_network().await {
                    Ok(_) => {
                        match with_timeout(
                            Duration::from_secs(180),
                            self.is_network_attached_loop(),
                        )
                        .await
                        {
                            Ok(_) => {
                                debug!("Will set Connected");
                                self.ch.set_power_state(OperationState::Connected);
                                debug!("Set Connected");
                            }
                            Err(err) => {
                                error!("Timeout waiting for network attach: {:?}", err);
                                return Err(Error::StateTimeout);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Error in init_network: {:?}", err);
                        return Err(err);
                    }
                },
                Ok(OperationState::DataEstablished) => {
                    match self.connect(C::APN, C::PROFILE_ID, C::CONTEXT_ID).await {
                        Ok(_) => {
                            self.ch.set_power_state(OperationState::DataEstablished);
                        }
                        Err(err) => {
                            error!("Error in connect: {:?}", err);
                            return Err(err);
                        }
                    }
                }
                Err(_) => {
                    error!("State transition next_state not valid: start_state={}, next_state={}, steps={} ", start_state, next_state, steps);
                    return Err(Error::InvalidStateTransition);
                }
            }
        }
        Ok(())
    }

    async fn handle_urc(&mut self, event: Urc) -> Result<(), Error> {
        match event {
            // Handle network URCs
            Urc::NetworkDetach => warn!("Network detached"),
            Urc::MobileStationDetach => warn!("Mobile station detached"),
            Urc::NetworkDeactivate => warn!("Network deactivated"),
            Urc::MobileStationDeactivate => warn!("Mobile station deactivated"),
            Urc::NetworkPDNDeactivate => warn!("Network PDN deactivated"),
            Urc::MobileStationPDNDeactivate => warn!("Mobile station PDN deactivated"),
            #[cfg(feature = "internal-network-stack")]
            Urc::SocketDataAvailable(_) => warn!("Socket data available"),
            #[cfg(feature = "internal-network-stack")]
            Urc::SocketDataAvailableUDP(_) => warn!("Socket data available UDP"),
            Urc::DataConnectionActivated(_) => warn!("Data connection activated"),
            Urc::DataConnectionDeactivated(_) => warn!("Data connection deactivated"),
            #[cfg(feature = "internal-network-stack")]
            Urc::SocketClosed(_) => warn!("Socket closed"),
            Urc::MessageWaitingIndication(_) => warn!("Message waiting indication"),
            Urc::ExtendedPSNetworkRegistration(_) => warn!("Extended PS network registration"),
            Urc::HttpResponse(_) => warn!("HTTP response"),
        };
        Ok(())
    }

    #[allow(unused_variables)]
    async fn connect(
        &mut self,
        apn_info: crate::config::Apn<'_>,
        profile_id: ProfileId,
        context_id: ContextId,
    ) -> Result<(), Error> {
        // This step _shouldn't_ be necessary.  However, for reasons I don't
        // understand, SARA-R4 can be registered but not attached (i.e. AT+CGATT
        // returns 0) on both RATs (unh?).  Phil Ware, who knows about these
        // things, always goes through (a) register, (b) wait for AT+CGATT to
        // return 1 and then (c) check that a context is active with AT+CGACT or
        // using AT+UPSD (even for EUTRAN). Since this sequence works for both
        // RANs, it is best to be consistent.
        let mut attached = false;
        for _ in 0..10 {
            attached = self.is_network_attached().await?;
            if attached {
                break;
            }
        }
        if !attached {
            return Err(Error::AttachTimeout);
        }

        // Activate the context
        #[cfg(feature = "upsd-context-activation")]
        self.activate_context_upsd(profile_id, apn_info).await?;
        #[cfg(not(feature = "upsd-context-activation"))]
        self.activate_context(context_id, profile_id).await?;

        Ok(())
    }

    // Make sure we are attached to the cellular network.
    async fn is_network_attached(&mut self) -> Result<bool, Error> {
        // Check for AT+CGATT to return 1
        let GPRSAttached { state } = self.at.send(&GetGPRSAttached).await?;

        if state == GPRSAttachedState::Attached {
            return Ok(true);
        }
        return Ok(false);

        // self.at .send( &SetGPRSAttached { state:
        //     GPRSAttachedState::Attached, } ).await .map_err(Error::from)?;
    }

    /// Activate context using AT+UPSD commands
    /// Required for SARA-G3, SARA-U2 SARA-R5 modules.
    #[cfg(feature = "upsd-context-activation")]
    async fn activate_context_upsd(
        &mut self,
        profile_id: ProfileId,
        apn_info: Apn<'_>,
    ) -> Result<(), Error> {
        // Check if the PSD profile is activated (param_tag = 1)
        let PacketSwitchedNetworkData { param_tag, .. } = self
            .at
            .send(&GetPacketSwitchedNetworkData {
                profile_id,
                param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
            })
            .await
            .map_err(Error::from)?;

        if param_tag == 0 {
            // SARA-U2 pattern: everything is done through AT+UPSD
            // Set up the APN
            if let Apn::Given {
                name,
                username,
                password,
            } = apn_info
            {
                self.at
                    .send(&SetPacketSwitchedConfig {
                        profile_id,
                        param: PacketSwitchedParam::APN(String::<99>::try_from(name).unwrap()),
                    })
                    .await
                    .map_err(Error::from)?;

                // Set up the user name
                if let Some(user_name) = username {
                    self.at
                        .send(&SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::Username(
                                String::<64>::try_from(user_name).unwrap(),
                            ),
                        })
                        .await
                        .map_err(Error::from)?;
                }

                // Set up the password
                if let Some(password) = password {
                    self.at
                        .send(&SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::Password(
                                String::<64>::try_from(password).unwrap(),
                            ),
                        })
                        .await
                        .map_err(Error::from)?;
                }
            }
            // Set up the dynamic IP address assignment.
            #[cfg(not(feature = "sara-r5"))]
            self.at
                .send(&SetPacketSwitchedConfig {
                    profile_id,
                    param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
                })
                .await
                .map_err(Error::from)?;

            // Automatic authentication protocol selection
            #[cfg(not(feature = "sara-r5"))]
            self.at
                .send(&SetPacketSwitchedConfig {
                    profile_id,
                    param: PacketSwitchedParam::Authentication(AuthenticationType::Auto),
                })
                .await
                .map_err(Error::from)?;

            #[cfg(not(feature = "sara-r5"))]
            self.at
                .send(&SetPacketSwitchedConfig {
                    profile_id,
                    param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
                })
                .await
                .map_err(Error::from)?;

            #[cfg(feature = "sara-r5")]
            self.at
                .send(&SetPacketSwitchedConfig {
                    profile_id,
                    param: PacketSwitchedParam::ProtocolType(ProtocolType::IPv4),
                })
                .await
                .map_err(Error::from)?;

            #[cfg(feature = "sara-r5")]
            self.at
                .send(&SetPacketSwitchedConfig {
                    profile_id,
                    param: PacketSwitchedParam::MapProfile(ContextId(1)),
                })
                .await
                .map_err(Error::from)?;

            self.at
                .send(&SetPacketSwitchedAction {
                    profile_id,
                    action: PacketSwitchedAction::Activate,
                })
                .await
                .map_err(Error::from)?;
        }

        Ok(())
    }

    /// Activate context using 3GPP commands
    /// Required for SARA-R4 and TOBY modules.
    #[cfg(not(feature = "upsd-context-activation"))]
    async fn activate_context(
        &mut self,
        cid: ContextId,
        _profile_id: ProfileId,
    ) -> Result<(), Error> {
        for _ in 0..10 {
            let context_states = self
                .at
                .send(&GetPDPContextState)
                .await
                .map_err(Error::from)?;

            let activated = context_states
                .iter()
                .find_map(|state| {
                    if state.cid == cid {
                        Some(state.status == PDPContextStatus::Activated)
                    } else {
                        None
                    }
                })
                .unwrap_or(false);

            if activated {
                // Note: SARA-R4 only supports a single context at any one time and
                // so doesn't require/support AT+UPSD.
                #[cfg(not(any(feature = "sara-r4", feature = "lara-r6")))]
                {
                    if let psn::responses::PacketSwitchedConfig {
                        param: psn::types::PacketSwitchedParam::MapProfile(context),
                        ..
                    } = self
                        .at
                        .send(&psn::GetPacketSwitchedConfig {
                            profile_id: _profile_id,
                            param: psn::types::PacketSwitchedParamReq::MapProfile,
                        })
                        .await
                        .map_err(Error::from)?
                    {
                        if context != cid {
                            self.at
                                .send(&psn::SetPacketSwitchedConfig {
                                    profile_id: _profile_id,
                                    param: psn::types::PacketSwitchedParam::MapProfile(cid),
                                })
                                .await
                                .map_err(Error::from)?;

                            self.at
                                .send(
                                    &psn::GetPacketSwitchedNetworkData {
                                        profile_id: _profile_id,
                                        param: psn::types::PacketSwitchedNetworkDataParam::PsdProfileStatus,
                                    },
                                ).await
                                .map_err(Error::from)?;
                        }
                    }

                    let psn::responses::PacketSwitchedNetworkData { param_tag, .. } = self
                        .at
                        .send(&psn::GetPacketSwitchedNetworkData {
                            profile_id: _profile_id,
                            param: psn::types::PacketSwitchedNetworkDataParam::PsdProfileStatus,
                        })
                        .await
                        .map_err(Error::from)?;

                    if param_tag == 0 {
                        self.at
                            .send(&psn::SetPacketSwitchedAction {
                                profile_id: _profile_id,
                                action: psn::types::PacketSwitchedAction::Activate,
                            })
                            .await
                            .map_err(Error::from)?;
                    }
                }

                return Ok(());
            } else {
                self.at
                    .send(&SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: Some(cid),
                    })
                    .await
                    .map_err(Error::from)?;
                Timer::after(Duration::from_secs(1)).await;
            }
        }
        return Err(Error::ContextActivationTimeout);
    }
}

pub(crate) async fn init_at<A: AtatClient>(
    at_client: &mut A,
    enable_flow_control: bool,
) -> Result<(), Error> {
    // Allow auto bauding to kick in
    embassy_time::with_timeout(boot_time() * 2, async {
        loop {
            if let Ok(alive) = at_client.send(&AT).await {
                break alive;
            }
            Timer::after(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| Error::PoweredDown)?;

    // Extended errors on
    at_client
        .send(&SetReportMobileTerminationError {
            n: TerminationErrorMode::Enabled,
        })
        .await?;

    // Echo off
    at_client.send(&SetEcho { enabled: Echo::Off }).await?;

    // Select SIM
    at_client
        .send(&SetGpioConfiguration {
            gpio_id: 25,
            gpio_mode: GpioMode::Output(GpioOutValue::High),
        })
        .await?;

    #[cfg(any(feature = "lara-r6"))]
    at_client
        .send(&SetGpioConfiguration {
            gpio_id: 42,
            gpio_mode: GpioMode::Input(GpioInPull::NoPull),
        })
        .await?;

    let _model_id = at_client.send(&GetModelId).await?;

    // at_client.send(
    //     &IdentificationInformation {
    //         n: 9
    //     },
    // ).await?;

    at_client.send(&GetFirmwareVersion).await?;

    select_sim_card(at_client).await?;

    let ccid = at_client.send(&GetCCID).await?;
    info!("CCID: {}", ccid.ccid);

    // DCD circuit (109) changes in accordance with the carrier
    at_client
        .send(&SetCircuit109Behaviour {
            value: Circuit109Behaviour::ChangesWithCarrier,
        })
        .await?;

    // Ignore changes to DTR
    at_client
        .send(&SetCircuit108Behaviour {
            value: Circuit108Behaviour::Ignore,
        })
        .await?;

    // Switch off UART power saving until it is integrated into this API
    at_client
        .send(&SetPowerSavingControl {
            mode: PowerSavingMode::Disabled,
            timeout: None,
        })
        .await?;

    #[cfg(feature = "internal-network-stack")]
    if C::HEX_MODE {
        at_client
            .send(&crate::command::ip_transport_layer::SetHexMode {
                hex_mode_disable: crate::command::ip_transport_layer::types::HexMode::Enabled,
            })
            .await?;
    } else {
        at_client
            .send(&crate::command::ip_transport_layer::SetHexMode {
                hex_mode_disable: crate::command::ip_transport_layer::types::HexMode::Disabled,
            })
            .await?;
    }

    // Tell module whether we support flow control
    if enable_flow_control {
        at_client.send(&SetFlowControl).await?;
    } else {
        at_client.send(&SetFlowControl).await?;
    }
    Ok(())
}

pub(crate) async fn select_sim_card<A: AtatClient>(at_client: &mut A) -> Result<(), Error> {
    for _ in 0..2 {
        match at_client.send(&GetPinStatus).await {
            Ok(PinStatus { code }) if code == PinStatusCode::Ready => {
                debug!("SIM is ready");
                return Ok(());
            }
            _ => {}
        }

        Timer::after(Duration::from_secs(1)).await;
    }

    // There was an error initializing the SIM
    // We've seen issues on uBlox-based devices, as a precation, we'll cycle
    // the modem here through minimal/full functional state.
    at_client
        .send(&SetModuleFunctionality {
            fun: Functionality::Minimum,
            // SARA-R5 This parameter can be used only when <fun> is 1, 4 or 19
            #[cfg(feature = "sara-r5")]
            rst: None,
            #[cfg(not(feature = "sara-r5"))]
            rst: Some(ResetMode::DontReset),
        })
        .await?;
    at_client
        .send(&SetModuleFunctionality {
            fun: Functionality::Full,
            rst: Some(ResetMode::DontReset),
        })
        .await?;

    Ok(())
}
