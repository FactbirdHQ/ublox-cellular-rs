use core::future::poll_fn;
use core::task::Poll;

use crate::command::control::types::Echo;
use crate::command::control::types::FlowControl;
use crate::command::control::SetEcho;

use crate::command::dns::ResolveNameIp;
use crate::command::general::GetCIMI;
use crate::command::general::IdentificationInformation;
use crate::command::mobile_control::responses::ModuleFunctionality;
use crate::command::mobile_control::types::PowerMode;
use crate::command::mobile_control::GetModuleFunctionality;
use crate::command::network_service::responses::OperatorSelection;
use crate::command::network_service::types::OperatorSelectionMode;
use crate::command::network_service::GetNetworkRegistrationStatus;
use crate::command::network_service::GetOperatorSelection;
use crate::command::network_service::SetChannelAndNetworkEnvDesc;
use crate::command::network_service::SetOperatorSelection;
use crate::command::psn;
use crate::command::psn::GetEPSNetworkRegistrationStatus;
use crate::command::psn::GetGPRSAttached;
use crate::command::psn::GetGPRSNetworkRegistrationStatus;
use crate::command::psn::GetPDPContextDefinition;
use crate::command::psn::GetPDPContextState;
use crate::command::psn::SetPDPContextState;

use crate::error::GenericError;
use crate::modules::Generic;
use crate::modules::Module;
use crate::modules::ModuleParams;
use crate::{command::Urc, config::CellularConfig};

use super::state;
use crate::asynch::state::OperationState;
use crate::command::control::types::{Circuit108Behaviour, Circuit109Behaviour};
use crate::command::control::{SetCircuit108Behaviour, SetCircuit109Behaviour, SetFlowControl};
use crate::command::device_lock::responses::PinStatus;
use crate::command::device_lock::types::PinStatusCode;
use crate::command::device_lock::GetPinStatus;
use crate::command::general::{GetCCID, GetModelId};
use crate::command::gpio::types::{GpioInPull, GpioMode, GpioOutValue};
use crate::command::gpio::SetGpioConfiguration;
use crate::command::mobile_control::types::{Functionality, TerminationErrorMode};
use crate::command::mobile_control::{SetModuleFunctionality, SetReportMobileTerminationError};
use crate::command::psn::responses::GPRSAttached;
use crate::command::psn::types::GPRSAttachedState;
use crate::command::psn::types::PDPContextStatus;
use crate::command::system_features::types::PowerSavingMode;
use crate::command::system_features::SetPowerSavingControl;
use crate::command::AT;
use crate::error::Error;

use atat::UrcChannel;
use atat::{asynch::AtatClient, UrcSubscription};
use embassy_futures::select::select;

use embassy_futures::select::Either3;
use embassy_time::{with_timeout, Duration, Timer};
use embedded_hal::digital::{InputPin, OutputPin};
use futures_util::FutureExt;

use crate::command::psn::types::{ContextId, ProfileId};
use embassy_futures::select::Either;

const GENERIC_PWR_ON_TIMES: [u16; 2] = [300, 2000];

pub struct NetDevice<'a, 'b, C, A> {
    ch: &'b state::Runner<'a>,
    config: &'b mut C,
    at_client: A,
}

impl<'a, 'b, C, A> NetDevice<'a, 'b, C, A>
where
    C: CellularConfig<'a>,
    A: AtatClient,
{
    pub fn new(ch: &'b state::Runner<'a>, config: &'b mut C, at_client: A) -> Self {
        Self {
            ch,
            config,
            at_client,
        }
    }

    pub async fn is_alive(&mut self) -> Result<bool, Error> {
        if !self.has_power().await? {
            return Err(Error::PoweredDown);
        }

        match self.at_client.send(&AT).await {
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
            for generic_time in GENERIC_PWR_ON_TIMES {
                let pull_time = self
                    .ch
                    .module()
                    .map(|m| m.power_on_pull_time())
                    .unwrap_or(Generic.power_on_pull_time())
                    .unwrap_or(Duration::from_millis(generic_time as _));
                if let Some(pin) = self.config.power_pin() {
                    pin.set_low().map_err(|_| Error::IoPin)?;
                    Timer::after(pull_time).await;
                    pin.set_high().map_err(|_| Error::IoPin)?;

                    Timer::after(
                        self.ch
                            .module()
                            .map(|m| m.boot_wait())
                            .unwrap_or(Generic.boot_wait()),
                    )
                    .await;

                    if !self.has_power().await? {
                        if self.ch.module().is_some() {
                            return Err(Error::PoweredDown);
                        }
                        continue;
                    }

                    self.ch.set_operation_state(OperationState::PowerUp);
                    debug!("Powered up");
                    return Ok(());
                } else {
                    warn!("No power pin configured");
                    return Ok(());
                }
            }
            Err(Error::PoweredDown)
        } else {
            Ok(())
        }
    }

    pub async fn wait_for_desired_state(&mut self, ps: OperationState) {
        self.ch.clone().wait_for_desired_state(ps).await
    }

    pub async fn power_down(&mut self) -> Result<(), Error> {
        if self.has_power().await? {
            if let Some(pin) = self.config.power_pin() {
                pin.set_low().map_err(|_| Error::IoPin)?;
                Timer::after(
                    self.ch
                        .module()
                        .map(|m| m.power_off_pull_time())
                        .unwrap_or(Generic.power_off_pull_time()),
                )
                .await;
                pin.set_high().map_err(|_| Error::IoPin)?;
                self.ch.set_operation_state(OperationState::PowerDown);
                debug!("Powered down");

                Timer::after(Duration::from_secs(1)).await;

                Ok(())
            } else {
                warn!("No power pin configured");
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    /// Register with the cellular network
    ///
    /// # Errors
    ///
    /// Returns an error if any of the internal network operations fail.
    ///
    pub async fn register_network(&mut self, mcc_mnc: Option<()>) -> Result<(), Error> {
        self.prepare_connect().await?;

        if mcc_mnc.is_none() {
            // If no MCC/MNC is given, make sure we are in automatic network
            // selection mode.

            // Set automatic operator selection, if not already set
            let OperatorSelection { mode, .. } = self.at_client.send(&GetOperatorSelection).await?;

            if mode != OperatorSelectionMode::Automatic {
                // Don't check error code here as some modules can
                // return an error as we still have the radio off (but they still
                // obey)
                let _ = self
                    .at_client
                    .send(&SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                        format: None,
                    })
                    .await;
            }
        }

        // Reset the current registration status
        self.ch.update_registration_with(|f| f.reset());

        self.at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            })
            .await?;

        if let Some(_) = mcc_mnc {
            // TODO: If MCC & MNC is set, register with manual operator selection.
            // This is currently not supported!

            // let crate::command::network_service::responses::OperatorSelection { mode, .. } = self
            //     .at_client
            //     .send(&crate::command::network_service::GetOperatorSelection)
            //     .await?;

            // // Only run AT+COPS=0 if currently de-registered, to avoid PLMN reselection
            // if !matches!(
            //     mode,
            //     crate::command::network_service::types::OperatorSelectionMode::Automatic
            //         | crate::command::network_service::types::OperatorSelectionMode::Manual
            // ) {
            //     self.at_client
            //         .send(&crate::command::network_service::SetOperatorSelection {
            //             mode: crate::command::network_service::types::OperatorSelectionMode::Automatic,
            //             format: Some(C::OPERATOR_FORMAT as u8),
            //         })
            //         .await?;
            // }
            unimplemented!()
        }

        Ok(())
    }

    pub(crate) async fn prepare_connect(&mut self) -> Result<(), Error> {
        // CREG URC
        self.at_client.send(
            &crate::command::network_service::SetNetworkRegistrationStatus {
                n: crate::command::network_service::types::NetworkRegistrationUrcConfig::UrcEnabled,
            }).await?;

        // CGREG URC
        self.at_client
            .send(&crate::command::psn::SetGPRSNetworkRegistrationStatus {
                n: crate::command::psn::types::GPRSNetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await?;

        // CEREG URC
        self.at_client
            .send(&crate::command::psn::SetEPSNetworkRegistrationStatus {
                n: crate::command::psn::types::EPSNetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await?;

        for _ in 0..10 {
            if self.at_client.send(&GetCIMI).await.is_ok() {
                break;
            }

            Timer::after(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    /// Reset the module by driving it's `RESET_N` pin low for 50 ms
    ///
    /// **NOTE** This function will reset NVM settings!
    pub async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Cellular Module");
        if let Some(pin) = self.config.reset_pin() {
            pin.set_low().ok();
            Timer::after(
                self.ch
                    .module()
                    .map(|m| m.reset_hold())
                    .unwrap_or(Generic.reset_hold()),
            )
            .await;
            pin.set_high().ok();
            Timer::after(
                self.ch
                    .module()
                    .map(|m| m.boot_wait())
                    .unwrap_or(Generic.boot_wait()),
            )
            .await;
            // self.is_alive().await?;
        } else {
            warn!("No reset pin configured");
        }
        Ok(())
    }

    /// Perform at full factory reset of the module, clearing all NVM sectors in the process
    pub async fn factory_reset(&mut self) -> Result<(), Error> {
        self.at_client
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
            .at_client
            .send(&SetModuleFunctionality { fun, rst: None })
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

    /// Wait until module is alive (uses `Vint` & `AT` command)
    async fn wait_alive(&mut self, timeout: Duration) -> Result<bool, Error> {
        let fut = async {
            loop {
                if let Ok(alive) = self.is_alive().await {
                    return alive;
                }
                Timer::after(Duration::from_millis(100)).await;
            }
        };
        Ok(embassy_time::with_timeout(timeout, fut).await?)
    }

    /// Check if we are registered to a network technology (uses +CxREG family
    /// commands)
    async fn wait_network_registered(&mut self, timeout: Duration) -> Result<(), Error> {
        let state_runner = self.ch.clone();
        let update_fut = async {
            loop {
                self.update_registration().await;

                Timer::after(Duration::from_millis(300)).await;
            }
        };

        Ok(embassy_time::with_timeout(
            timeout,
            select(
                update_fut,
                poll_fn(|cx| match state_runner.is_registered(Some(cx)) {
                    true => Poll::Ready(()),
                    false => Poll::Pending,
                }),
            ),
        )
        .await
        .map(drop)?)
    }

    async fn update_registration(&mut self) {
        if let Ok(reg) = self.at_client.send(&GetNetworkRegistrationStatus).await {
            self.ch
                .update_registration_with(|state| state.compare_and_set(reg.into()));
        }

        if let Ok(reg) = self.at_client.send(&GetGPRSNetworkRegistrationStatus).await {
            self.ch
                .update_registration_with(|state| state.compare_and_set(reg.into()));
        }

        if let Ok(reg) = self.at_client.send(&GetEPSNetworkRegistrationStatus).await {
            self.ch
                .update_registration_with(|state| state.compare_and_set(reg.into()));
        }
    }

    async fn init_at(&mut self) -> Result<(), Error> {
        // Allow auto bauding to kick in
        embassy_time::with_timeout(
            self.ch
                .module()
                .map(|m| m.boot_wait())
                .unwrap_or(Generic.boot_wait())
                * 2,
            async {
                loop {
                    if let Ok(alive) = self.at_client.send(&AT).await {
                        break alive;
                    }
                    Timer::after(Duration::from_millis(100)).await;
                }
            },
        )
        .await
        .map_err(|_| Error::PoweredDown)?;

        let model_id = self.at_client.send(&GetModelId).await?;
        self.ch.set_module(Module::from_model_id(model_id));

        // Echo off
        self.at_client.send(&SetEcho { enabled: Echo::Off }).await?;

        // Extended errors on
        self.at_client
            .send(&SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            })
            .await?;

        #[cfg(feature = "internal-network-stack")]
        if C::HEX_MODE {
            self.at_client
                .send(&crate::command::ip_transport_layer::SetHexMode {
                    hex_mode_disable: crate::command::ip_transport_layer::types::HexMode::Enabled,
                })
                .await?;
        } else {
            self.at_client
                .send(&crate::command::ip_transport_layer::SetHexMode {
                    hex_mode_disable: crate::command::ip_transport_layer::types::HexMode::Disabled,
                })
                .await?;
        }

        // FIXME: The following three GPIO settings should not be here!
        self.at_client
            .send(&SetGpioConfiguration {
                gpio_id: 23,
                gpio_mode: GpioMode::NetworkStatus,
            })
            .await;

        // Select SIM
        self.at_client
            .send(&SetGpioConfiguration {
                gpio_id: 25,
                gpio_mode: GpioMode::Output(GpioOutValue::Low),
            })
            .await?;

        #[cfg(feature = "lara-r6")]
        self.at_client
            .send(&SetGpioConfiguration {
                gpio_id: 42,
                gpio_mode: GpioMode::Input(GpioInPull::NoPull),
            })
            .await?;

        // self.at_client
        //     .send(&IdentificationInformation { n: 9 })
        //     .await?;

        // DCD circuit (109) changes in accordance with the carrier
        self.at_client
            .send(&SetCircuit109Behaviour {
                value: Circuit109Behaviour::AlwaysPresent,
            })
            .await?;

        // Ignore changes to DTR
        self.at_client
            .send(&SetCircuit108Behaviour {
                value: Circuit108Behaviour::Ignore,
            })
            .await?;

        self.check_sim_status().await?;

        let ccid = self.at_client.send(&GetCCID).await?;
        info!("CCID: {}", ccid.ccid);

        #[cfg(all(
            feature = "ucged",
            any(
                feature = "sara-r410m",
                feature = "sara-r412m",
                feature = "sara-r422",
                feature = "lara-r6"
            )
        ))]
        self.at_client
            .send(&SetChannelAndNetworkEnvDesc {
                mode: if cfg!(feature = "ucged5") { 5 } else { 2 },
            })
            .await?;

        // Tell module whether we support flow control
        if C::FLOW_CONTROL {
            self.at_client
                .send(&SetFlowControl {
                    value: FlowControl::RtsCts,
                })
                .await?;
        } else {
            self.at_client
                .send(&SetFlowControl {
                    value: FlowControl::Disabled,
                })
                .await?;
        }

        // Switch off UART power saving until it is integrated into this API
        self.at_client
            .send(&SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
            })
            .await?;

        if !self.ch.is_registered(None) {
            self.at_client
                .send(&SetModuleFunctionality {
                    fun: self
                        .ch
                        .module()
                        .ok_or(Error::Uninitialized)?
                        .radio_off_cfun(),
                    rst: None,
                })
                .await?;
        }

        Ok(())
    }

    async fn radio_off(&mut self) -> Result<(), Error> {
        #[cfg(not(feature = "use-upsd-context-activation"))]
        self.ch
            .set_profile_state(crate::registration::ProfileState::ShouldBeDown);

        let module_cfun = self
            .ch
            .module()
            .ok_or(Error::Uninitialized)?
            .radio_off_cfun();

        let cfun_power_mode = PowerMode::try_from(module_cfun as u8).ok();

        let mut last_err = None;
        for _ in 0..3 {
            match self
                .at_client
                .send(&SetModuleFunctionality {
                    fun: module_cfun,
                    rst: None,
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_err.replace(e);

                    if let Some(expected_mode) = cfun_power_mode {
                        match self.at_client.send(&GetModuleFunctionality).await {
                            Ok(ModuleFunctionality { power_mode, .. })
                                if power_mode == expected_mode =>
                            {
                                // If we got no response, abort the command and
                                // check the status
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Err(last_err.unwrap().into())
    }

    async fn check_sim_status(&mut self) -> Result<(), Error> {
        for _ in 0..2 {
            match self.at_client.send(&GetPinStatus).await {
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
        self.at_client
            .send(&SetModuleFunctionality {
                fun: self
                    .ch
                    .module()
                    .ok_or(Error::Uninitialized)?
                    .radio_off_cfun(),
                rst: None,
            })
            .await?;
        self.at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            })
            .await?;

        Ok(())
    }

    pub async fn run(&mut self) -> ! {
        match self.has_power().await {
            Ok(true) => {
                self.ch.set_operation_state(OperationState::PowerUp);
            }
            Ok(false) | Err(_) => {
                self.ch.set_operation_state(OperationState::PowerDown);
            }
        }

        loop {
            // FIXME: This does not seem to work as expected?
            match embassy_futures::select::select(
                self.ch.clone().wait_for_desired_state_change(),
                self.ch.clone().wait_registration_change(),
            )
            .await
            {
                Either::First(desired_state) => {
                    info!("Desired state: {:?}", desired_state);
                    let _ = self.run_to_state(desired_state).await;
                }
                Either::Second(false) => {
                    warn!("Lost network registration. Setting operating state back to initialized");

                    self.ch.set_operation_state(OperationState::Initialized);
                    let _ = self
                        .run_to_state(self.ch.clone().operation_state(None))
                        .await;
                }
                Either::Second(true) => {
                    // This flag will be set if we had been knocked out
                    // of our PDP context by a network outage and need
                    // to get it back again; make sure to get this in the
                    // queue before any user registratioon status callback
                    // so that everything is sorted for them
                    #[cfg(not(feature = "use-upsd-context-activation"))]
                    if self.ch.get_profile_state()
                        == crate::registration::ProfileState::RequiresReactivation
                    {
                        self.activate_context(C::CONTEXT_ID, C::PROFILE_ID)
                            .await
                            .unwrap();
                        self.ch
                            .set_profile_state(crate::registration::ProfileState::ShouldBeUp);
                    }
                }
                _ => {}
            }
        }
    }

    pub async fn run_to_state(&mut self, desired_state: OperationState) -> Result<(), Error> {
        if 0 >= desired_state as isize - self.ch.clone().operation_state(None) as isize {
            debug!(
                "Power steps was negative, power down: {}",
                desired_state as isize - self.ch.clone().operation_state(None) as isize
            );
            self.power_down().await.ok();
            self.ch.set_operation_state(OperationState::PowerDown);
        }
        let start_state = self.ch.clone().operation_state(None) as isize;
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
                        self.ch.set_operation_state(OperationState::PowerUp);
                    }
                    Err(err) => {
                        error!("Error in power_up: {:?}", err);
                        return Err(err);
                    }
                },
                Ok(OperationState::Initialized) => match self.init_at().await {
                    Ok(_) => {
                        self.ch.set_operation_state(OperationState::Initialized);
                    }
                    Err(err) => {
                        error!("Error in init_at: {:?}", err);
                        return Err(err);
                    }
                },
                Ok(OperationState::Connected) => match self.register_network(None).await {
                    Ok(_) => match self.wait_network_registered(Duration::from_secs(180)).await {
                        Ok(_) => {
                            self.ch.set_operation_state(OperationState::Connected);
                        }
                        Err(err) => {
                            error!("Timeout waiting for network attach: {:?}", err);
                            return Err(err);
                        }
                    },
                    Err(err) => {
                        error!("Error in register_network: {:?}", err);
                        return Err(err);
                    }
                },
                Ok(OperationState::DataEstablished) => {
                    match self.connect(C::APN, C::PROFILE_ID, C::CONTEXT_ID).await {
                        Ok(_) => {
                            #[cfg(not(feature = "use-upsd-context-activation"))]
                            self.ch
                                .set_profile_state(crate::registration::ProfileState::ShouldBeUp);

                            self.ch.set_operation_state(OperationState::DataEstablished);
                            Timer::after(Duration::from_secs(5)).await;
                        }
                        Err(err) => {
                            // Switch radio off after failure
                            let _ = self.radio_off().await;

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

    #[allow(unused_variables)]
    async fn connect(
        &mut self,
        apn_info: crate::config::Apn<'_>,
        profile_id: ProfileId,
        context_id: ContextId,
    ) -> Result<(), Error> {
        #[cfg(not(feature = "use-upsd-context-activation"))]
        self.define_context(context_id, apn_info).await?;

        // This step _shouldn't_ be necessary.  However, for reasons I don't
        // understand, SARA-R4 can be registered but not attached (i.e. AT+CGATT
        // returns 0) on both RATs (unh?).  Phil Ware, who knows about these
        // things, always goes through (a) register, (b) wait for AT+CGATT to
        // return 1 and then (c) check that a context is active with AT+CGACT or
        // using AT+UPSD (even for EUTRAN). Since this sequence works for both
        // RANs, it is best to be consistent.
        let mut attached = false;
        for _ in 0..10 {
            if let Ok(true) = self.is_network_attached().await {
                attached = true;
                break;
            };
            Timer::after(Duration::from_secs(1)).await;
        }
        if !attached {
            return Err(Error::AttachTimeout);
        }

        // Activate the context
        #[cfg(feature = "use-upsd-context-activation")]
        self.activate_context_upsd(profile_id, apn_info).await?;
        #[cfg(not(feature = "use-upsd-context-activation"))]
        self.activate_context(context_id, profile_id).await?;

        Ok(())
    }

    /// Define a PDP context
    #[cfg(not(feature = "use-upsd-context-activation"))]
    async fn define_context(
        &mut self,
        cid: ContextId,
        apn_info: crate::config::Apn<'_>,
    ) -> Result<(), Error> {
        use crate::command::psn::{
            types::AuthenticationType, SetAuthParameters, SetPDPContextDefinition,
        };

        self.at_client
            .send(&SetModuleFunctionality {
                fun: self
                    .ch
                    .module()
                    .ok_or(Error::Uninitialized)?
                    .radio_off_cfun(),
                rst: None,
            })
            .await?;

        if let crate::config::Apn::Given {
            name,
            username,
            password,
        } = apn_info
        {
            self.at_client
                .send(&SetPDPContextDefinition {
                    cid,
                    pdp_type: "IP",
                    apn: name,
                })
                .await?;

            if let Some(username) = username {
                self.at_client
                    .send(&SetAuthParameters {
                        cid,
                        auth_type: AuthenticationType::Auto,
                        username,
                        password: password.unwrap_or_default(),
                    })
                    .await?;
            }
        }

        self.at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            })
            .await?;

        Ok(())
    }

    // Make sure we are attached to the cellular network.
    async fn is_network_attached(&mut self) -> Result<bool, Error> {
        // Check for AT+CGATT to return 1
        let GPRSAttached { state } = self.at_client.send(&GetGPRSAttached).await?;
        Ok(state == GPRSAttachedState::Attached)
    }

    /// Activate context using AT+UPSD commands.
    #[cfg(feature = "use-upsd-context-activation")]
    async fn activate_context_upsd(
        &mut self,
        profile_id: ProfileId,
        apn_info: crate::config::Apn<'_>,
    ) -> Result<(), Error> {
        // SARA-U2 pattern: everything is done through AT+UPSD
        // Set up the APN
        if let crate::config::Apn::Given {
            name,
            username,
            password,
        } = apn_info
        {
            self.at_client
                .send(&psn::SetPacketSwitchedConfig {
                    profile_id,
                    param: psn::types::PacketSwitchedParam::APN(
                        String::<99>::try_from(name).unwrap(),
                    ),
                })
                .await?;

            // Set up the user name
            if let Some(user_name) = username {
                self.at_client
                    .send(&psn::SetPacketSwitchedConfig {
                        profile_id,
                        param: psn::types::PacketSwitchedParam::Username(
                            String::<64>::try_from(user_name).unwrap(),
                        ),
                    })
                    .await?;
            }

            // Set up the password
            if let Some(password) = password {
                self.at_client
                    .send(&psn::SetPacketSwitchedConfig {
                        profile_id,
                        param: psn::types::PacketSwitchedParam::Password(
                            String::<64>::try_from(password).unwrap(),
                        ),
                    })
                    .await?;
            }
        }
        // Set up the dynamic IP address assignment.
        self.at_client
            .send(&psn::SetPacketSwitchedConfig {
                profile_id,
                param: psn::types::PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
            })
            .await?;

        // Automatic authentication protocol selection
        self.at_client
            .send(&psn::SetPacketSwitchedConfig {
                profile_id,
                param: psn::types::PacketSwitchedParam::Authentication(AuthenticationType::Auto),
            })
            .await?;

        self.at_client
            .send(&psn::SetPacketSwitchedAction {
                profile_id,
                action: psn::types::PacketSwitchedAction::Activate,
            })
            .await?;

        Ok(())
    }

    /// Activate context using 3GPP commands
    #[cfg(not(feature = "use-upsd-context-activation"))]
    async fn activate_context(
        &mut self,
        cid: ContextId,
        _profile_id: ProfileId,
    ) -> Result<(), Error> {
        for _ in 0..5 {
            #[cfg(feature = "sara-r422")]
            {
                // Note: it seems a bit strange to do this first,
                // rather than just querying the +CGACT status,
                // but a specific case has been found where SARA-R422
                // indicated that it was activated whereas in fact,
                // at least for the internal clients (so sockets, HTTP
                // and MQTT), it was not.  Forcing with AT+CGACT=1,x has
                // been shown to fix that.  We don't do it in all
                // cases as SARA-R41x modules object to that.
                self.at_client
                    .send(&SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: Some(cid),
                    })
                    .await?;
            }

            let context_states = self.at_client.send(&GetPDPContextState).await?;

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
                // [Re]attach a PDP context to an internal module profile
                #[cfg(feature = "context-mapping-required")]
                {
                    self.at_client
                        .send(&psn::SetPacketSwitchedConfig {
                            profile_id: _profile_id,
                            param: psn::types::PacketSwitchedParam::ProtocolType(
                                psn::types::ProtocolType::IPv4,
                            ),
                        })
                        .await?;

                    self.at_client
                        .send(&psn::SetPacketSwitchedConfig {
                            profile_id: _profile_id,
                            param: psn::types::PacketSwitchedParam::MapProfile(cid),
                        })
                        .await?;

                    // SARA-R5 pattern: the context also has to be
                    // activated and we're not actually done
                    // until the +UUPSDA URC comes back,
                    #[cfg(feature = "sara-r5")]
                    self.at_client
                        .send(&psn::SetPacketSwitchedAction {
                            profile_id,
                            action: psn::types::PacketSwitchedAction::Activate,
                        })
                        .await?;
                }

                return Ok(());
            } else {
                #[cfg(not(feature = "sara-r422"))]
                self.at_client
                    .send(&SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: Some(cid),
                    })
                    .await?;
            }
        }
        Err(Error::ContextActivationTimeout)
    }
}
