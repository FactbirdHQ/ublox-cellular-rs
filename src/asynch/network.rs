use core::{cmp::Ordering, future::poll_fn, marker::PhantomData, task::Poll};

#[cfg(feature = "use-upsd-context-activation")]
use core::net::Ipv4Addr;

use crate::{
    asynch::state::OperationState,
    command::{
        general::GetCIMI,
        mobile_control::{
            responses::ModuleFunctionality,
            types::{Functionality, PowerMode},
            GetExtendedErrorReport, GetModuleFunctionality, SetModuleFunctionality,
        },
        network_service::{
            responses::OperatorSelection,
            types::{NetworkRegistrationUrcConfig, OperatorSelectionMode},
            GetNetworkRegistrationStatus, GetOperatorSelection, SetNetworkRegistrationStatus,
            SetOperatorSelection,
        },
        psn::{
            responses::GPRSAttached,
            types::{
                ContextId, EPSNetworkRegistrationUrcConfig, GPRSAttachedState,
                GPRSNetworkRegistrationUrcConfig, PDPContextStatus, ProfileId,
            },
            GetEPSNetworkRegistrationStatus, GetGPRSAttached, GetGPRSNetworkRegistrationStatus,
            GetPDPContextState, SetEPSNetworkRegistrationStatus, SetGPRSNetworkRegistrationStatus,
            SetPDPContextState,
        },
    },
    config::{Apn, CellularConfig},
    error::Error,
    modules::ModuleParams,
    registration::ProfileState,
};

use super::state;

use atat::asynch::AtatClient;
use embassy_futures::select::{select, Either};

use embassy_time::{Duration, Timer};

pub struct NetDevice<'a, 'b, C, A> {
    ch: &'b state::Runner<'a>,
    at_client: A,
    _config: PhantomData<C>,
}

impl<'a, 'b, C, A> NetDevice<'a, 'b, C, A>
where
    C: CellularConfig<'a>,
    A: AtatClient,
{
    pub fn new(ch: &'b state::Runner<'a>, at_client: A) -> Self {
        Self {
            ch,
            at_client,
            _config: PhantomData,
        }
    }

    /// Register with the cellular network
    ///
    /// # Errors
    ///
    /// Returns an error if any of the internal network operations fail.
    ///
    async fn register_network(&mut self, mcc_mnc: Option<()>) -> Result<(), Error> {
        info!("🔧 NetDevice::register_network() - Starting network registration process");
        debug!(
            "NetDevice::register_network() - MCC/MNC parameter: {:?}",
            mcc_mnc
        );

        info!("NetDevice::register_network() - Calling prepare_connect()");
        self.prepare_connect().await?;
        info!("NetDevice::register_network() - prepare_connect() completed successfully");

        if mcc_mnc.is_none() {
            info!("NetDevice::register_network() - No MCC/MNC specified, setting automatic network selection");
            // If no MCC/MNC is given, make sure we are in automatic network
            // selection mode.

            // Set automatic operator selection, if not already set
            debug!("NetDevice::register_network() - Getting current operator selection");
            let OperatorSelection { mode, .. } = self.at_client.send(&GetOperatorSelection).await?;
            info!(
                "NetDevice::register_network() - Current operator selection mode: {:?}",
                mode
            );

            if mode != OperatorSelectionMode::Automatic {
                info!("NetDevice::register_network() - Switching to automatic operator selection mode");
                // Don't check error code here as some modules can
                // return an error as we still have the radio off (but they still
                // obey)
                match self
                    .at_client
                    .send(&SetOperatorSelection {
                        mode: OperatorSelectionMode::Automatic,
                        format: None,
                    })
                    .await {
                    Ok(_) => info!("NetDevice::register_network() - Successfully set automatic operator selection"),
                    Err(e) => warn!("NetDevice::register_network() - Failed to set automatic operator selection (this may be expected): {:?}", e)
                }
            } else {
                info!(
                    "NetDevice::register_network() - Already in automatic operator selection mode"
                );
            }
        }

        // Reset the current registration status
        info!("NetDevice::register_network() - Resetting registration status");
        self.ch.update_registration_with(|f| {
            f.reset();
            false // Reset doesn't constitute a RAT change
        });
        debug!("NetDevice::register_network() - Registration status reset completed");

        // Pre-configure PDP context with APN before enabling radio.
        // For LTE, the Initial Default EPS Bearer Activation happens during
        // the Attach procedure — the APN must be defined beforehand.
        // For 2G/3G this is harmless: CGDCONT just stores parameters for
        // later device-initiated PDP context activation.
        #[cfg(not(feature = "use-upsd-context-activation"))]
        {
            let apn_info = self.ch.get_apn_config();
            if let crate::config::Apn::Given {
                name,
                username,
                password,
            } = apn_info
            {
                use crate::command::psn::{
                    types::AuthenticationType, SetAuthParameters, SetPDPContextDefinition,
                };

                // Ensure radio is off (CGDCONT only takes effect on next attach)
                let _ = self
                    .at_client
                    .send(&SetModuleFunctionality {
                        fun: self
                            .ch
                            .module()
                            .ok_or(Error::Uninitialized)?
                            .radio_off_cfun(),
                        rst: None,
                    })
                    .await;

                info!(
                    "Defining PDP context before registration: APN={}",
                    name.as_str()
                );
                self.at_client
                    .send(&SetPDPContextDefinition {
                        cid: C::CONTEXT_ID,
                        pdp_type: "IP",
                        apn: name.as_str(),
                    })
                    .await?;

                if let Some(username) = username {
                    self.at_client
                        .send(&SetAuthParameters {
                            cid: C::CONTEXT_ID,
                            auth_type: AuthenticationType::Auto,
                            username: username.as_str(),
                            password: password.unwrap_or_default().as_str(),
                        })
                        .await?;
                }
            }
        }

        info!("NetDevice::register_network() - Setting module functionality to Full");
        match self
            .at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Full,
                rst: None,
            })
            .await
        {
            Ok(_) => info!(
                "NetDevice::register_network() - Successfully set module functionality to Full"
            ),
            Err(e) => {
                error!(
                    "NetDevice::register_network() - Failed to set module functionality: {:?}",
                    e
                );
                return Err(e.into());
            }
        }

        if mcc_mnc.is_some() {
            error!("NetDevice::register_network() - Manual operator selection with MCC/MNC is not implemented!");
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

        info!("✅ NetDevice::register_network() - Network registration completed successfully");
        Ok(())
    }

    async fn prepare_connect(&mut self) -> Result<(), Error> {
        info!("🔧 NetDevice::prepare_connect() - Starting connection preparation");

        // CREG URC
        debug!("NetDevice::prepare_connect() - Setting up CREG URC (Network Registration)");
        match self
            .at_client
            .send(&SetNetworkRegistrationStatus {
                n: NetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await
        {
            Ok(_) => info!("NetDevice::prepare_connect() - Successfully enabled CREG URC"),
            Err(e) => {
                error!(
                    "NetDevice::prepare_connect() - Failed to enable CREG URC: {:?}",
                    e
                );
                return Err(e.into());
            }
        }
        // CGREG URC
        debug!("NetDevice::prepare_connect() - Setting up CGREG URC (GPRS Registration)");
        match self
            .at_client
            .send(&SetGPRSNetworkRegistrationStatus {
                n: GPRSNetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await
        {
            Ok(_) => info!("NetDevice::prepare_connect() - Successfully enabled CGREG URC"),
            Err(e) => {
                error!(
                    "NetDevice::prepare_connect() - Failed to enable CGREG URC: {:?}",
                    e
                );
                return Err(e.into());
            }
        }

        // CEREG URC
        debug!("NetDevice::prepare_connect() - Setting up CEREG URC (EPS Registration)");
        match self
            .at_client
            .send(&SetEPSNetworkRegistrationStatus {
                n: EPSNetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await
        {
            Ok(_) => info!("NetDevice::prepare_connect() - Successfully enabled CEREG URC"),
            Err(e) => {
                error!(
                    "NetDevice::prepare_connect() - Failed to enable CEREG URC: {:?}",
                    e
                );
                return Err(e.into());
            }
        }

        info!("NetDevice::prepare_connect() - Starting module readiness check with CIMI command");
        let mut ready = false;
        for attempt in 0..10 {
            debug!(
                "NetDevice::prepare_connect() - CIMI attempt {}/{}",
                attempt + 1,
                10
            );
            match self.at_client.send(&GetCIMI).await {
                Ok(cimi_response) => {
                    info!(
                        "NetDevice::prepare_connect() - Module is ready! CIMI response: {:?}",
                        cimi_response.imsi
                    );
                    ready = true;
                    break;
                }
                Err(e) => {
                    warn!(
                        "NetDevice::prepare_connect() - CIMI attempt {}/{} failed: {:?}",
                        attempt + 1,
                        10,
                        e
                    );
                    if attempt < 9 {
                        debug!("NetDevice::prepare_connect() - Waiting 1 second before next CIMI attempt");
                        Timer::after_secs(1).await;
                    }
                }
            }
        }

        if !ready {
            error!("NetDevice::prepare_connect() - Module failed to respond to CIMI after 10 attempts!");
            return Err(Error::Generic(crate::error::GenericError::Timeout));
        }

        info!("✅ NetDevice::prepare_connect() - Connection preparation completed successfully");
        Ok(())
    }

    // Perform at full factory reset of the module, clearing all NVM sectors in the process
    // TODO: Should this be moved to control?
    // async fn factory_reset(&mut self) -> Result<(), Error> {
    //     self.at_client
    //         .send(&SetFactoryConfiguration {
    //             fs_op: FSFactoryRestoreType::NoRestore,
    //             nvm_op: NVMFactoryRestoreType::NVMFlashSectors,
    //         })
    //         .await?;

    //     info!("Successfully factory reset modem!");

    //     if self.soft_reset(true).await.is_err() {
    //         self.pwr.reset().await?;
    //     }

    //     Ok(())
    // }

    /// Reset the module by sending AT CFUN command
    async fn soft_reset(&mut self, sim_reset: bool) -> Result<(), Error> {
        debug!(
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

    /// Check if we are registered to a network technology (uses +CxREG family
    /// commands)
    async fn wait_network_registered(&mut self, timeout: Duration) -> Result<(), Error> {
        info!(
            "🔧 NetDevice::wait_network_registered() - Starting to wait for network registration (timeout: {:?})",
            timeout
        );

        let state_runner = self.ch.clone();

        let wait_fut = async {
            loop {
                debug!("NetDevice::wait_network_registered()");

                self.update_registration().await?;

                if state_runner.is_registered(None) {
                    info!("✅ NetDevice::wait_network_registered() - Successfully registered to network");
                    return Ok(());
                }

                Timer::after_secs(3).await;
            }
        };

        embassy_time::with_timeout(timeout, wait_fut)
            .await
            .map_err(|_| {
                error!(
                    "❌ NetDevice::wait_network_registered() - Failed to register within timeout"
                );
                Error::Generic(crate::error::GenericError::Timeout)
            })?
    }

    async fn update_registration(&mut self) -> Result<(), Error> {
        debug!("NetDevice::update_registration() - Checking all registration statuses");

        let mut any_ok = false;

        // Check Network Registration (CREG)
        match self.at_client.send(&GetNetworkRegistrationStatus).await {
            Ok(reg) => {
                debug!("NetDevice::update_registration() - CREG status: {:?}", reg);
                self.ch
                    .update_registration_with(|state| state.compare_and_set(reg.into()));
                any_ok = true;
            }
            Err(e) => {
                warn!(
                    "NetDevice::update_registration() - Failed to get CREG status: {:?}",
                    e
                );
            }
        }

        // Check GPRS Registration (CGREG)
        match self.at_client.send(&GetGPRSNetworkRegistrationStatus).await {
            Ok(reg) => {
                debug!("NetDevice::update_registration() - CGREG status: {:?}", reg);
                self.ch
                    .update_registration_with(|state| state.compare_and_set(reg.into()));
                any_ok = true;
            }
            Err(e) => {
                warn!(
                    "NetDevice::update_registration() - Failed to get CGREG status: {:?}",
                    e
                );
            }
        }

        // Check EPS Registration (CEREG)
        match self.at_client.send(&GetEPSNetworkRegistrationStatus).await {
            Ok(reg) => {
                debug!("NetDevice::update_registration() - CEREG status: {:?}", reg);
                let eps_not_registered = !matches!(
                    reg.stat,
                    crate::command::psn::types::EPSNetworkRegistrationStat::Registered
                        | crate::command::psn::types::EPSNetworkRegistrationStat::RegisteredRoaming
                );
                self.ch
                    .update_registration_with(|state| state.compare_and_set(reg.into()));
                any_ok = true;

                // If EPS is not registered, query CEER for the rejection cause
                if eps_not_registered {
                    match self.at_client.send(&GetExtendedErrorReport).await {
                        Ok(report) => {
                            warn!(
                                "CEER: type={}, cause={}, desc={}",
                                report.r#type.as_str(),
                                report.cause,
                                report.description.as_str()
                            );
                        }
                        Err(e) => {
                            debug!("Failed to get CEER: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    "NetDevice::update_registration() - Failed to get CEREG status: {:?}",
                    e
                );
            }
        }

        if !any_ok {
            // All registration queries failed (likely SIM busy). Back off to
            // give the SIM time to complete internal operations instead of
            // hammering it every poll cycle.
            warn!("NetDevice::update_registration() - All registration queries failed, backing off 5s");
            Timer::after_secs(5).await;
        }

        Ok(())
    }

    async fn radio_off(&mut self) -> Result<(), Error> {
        #[cfg(not(feature = "use-upsd-context-activation"))]
        self.ch.set_profile_state(ProfileState::ShouldBeDown);

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

    pub async fn run(&mut self) -> Result<(), Error> {
        self.run_to_desired().await?;

        loop {
            match select(
                self.ch.wait_for_desired_state_change(),
                self.ch.wait_registration_change(),
            )
            .await
            {
                Either::First(_) => {
                    info!("desired state chagne, run to desired state");
                    self.run_to_desired().await?;
                }
                Either::Second(false) => {
                    warn!("Lost network registration. Setting operating state back to initialized");
                    self.ch.set_operation_state(OperationState::Initialized);
                    self.run_to_desired().await?;
                }
                Either::Second(true) => {
                    info!("Network registration changed");
                    // This flag will be set if we had been knocked out
                    // of our PDP context by a network outage and need
                    // to get it back again; make sure to get this in the
                    // queue before any user registratioon status callback
                    // so that everything is sorted for them
                    #[cfg(not(feature = "use-upsd-context-activation"))]
                    if self.ch.get_profile_state() == ProfileState::RequiresReactivation {
                        self.activate_context(C::CONTEXT_ID, C::PROFILE_ID).await?;
                        self.ch.set_profile_state(ProfileState::ShouldBeUp);
                    }
                }
            }
        }
    }

    async fn run_to_desired(&mut self) -> Result<(), Error> {
        loop {
            let current_state = self.ch.operation_state(None);
            let desired_state = self.ch.desired_state(None);

            debug!(
                "State transition: {:?} -> {:?}",
                current_state, desired_state
            );

            match (current_state, desired_state.cmp(&current_state)) {
                (_, Ordering::Equal) => break,

                (OperationState::PowerDown, Ordering::Greater) => {
                    self.ch
                        .wait_for_operation_state(OperationState::Initialized)
                        .await
                }
                (OperationState::Initialized, Ordering::Greater) => {
                    info!(
                        "NetDevice::run_to_desired() - Transitioning from Initialized to Connected"
                    );
                    debug!("NetDevice::run_to_desired() - Starting network registration process");

                    self.register_network(None).await?;
                    info!("NetDevice::run_to_desired() - Network registration completed, waiting for registration confirmation");

                    self.wait_network_registered(Duration::from_secs(180))
                        .await?;

                    info!("NetDevice::run_to_desired() - Network registration confirmed, setting state to Connected");
                    self.ch.set_operation_state(OperationState::Connected);
                }
                (OperationState::Connected, Ordering::Greater) => {
                    info!("NetDevice::run_to_desired() - Transitioning from Connected to DataEstablished");
                    info!("NetDevice::run_to_desired() - Operation state is connected, establishing data connection");

                    match self.connect(C::PROFILE_ID, C::CONTEXT_ID).await {
                        Ok(_) => {
                            info!("NetDevice::run_to_desired() - Data connection established successfully");
                            #[cfg(not(feature = "use-upsd-context-activation"))]
                            self.ch
                                .set_profile_state(crate::registration::ProfileState::ShouldBeUp);

                            self.ch.set_operation_state(OperationState::DataEstablished);
                            info!("NetDevice::run_to_desired() - State set to DataEstablished");
                        }
                        Err(err) => {
                            error!("NetDevice::run_to_desired() - Failed to establish data connection: {:?}", err);
                            // Switch radio off after failure
                            warn!("NetDevice::run_to_desired() - Switching radio off after connection failure");
                            let _ = self.radio_off().await;
                            return Err(err);
                        }
                    }
                }

                (OperationState::Connected, Ordering::Less) => {
                    // Deregister from network to clear cached NAS/EPS
                    // security context. Without this, the modem retains
                    // stale credentials in NVM across power cycles, causing
                    // the next registration attempt to fail (e.g. when
                    // switching SIMs — the modem tries a TAU with the old
                    // SIM's keys instead of a fresh IMSI attach).
                    let _ = self
                        .at_client
                        .send(&SetOperatorSelection {
                            mode: OperatorSelectionMode::Deregister,
                            format: None,
                        })
                        .await;
                    self.radio_off().await?;
                    self.ch.set_operation_state(OperationState::Initialized);
                }
                (OperationState::DataEstablished, Ordering::Less) => {
                    self.ch.set_operation_state(OperationState::Connected);
                }

                (OperationState::DataEstablished, Ordering::Greater) => unreachable!(),
                (OperationState::Initialized, Ordering::Less) => return Err(Error::PoweredDown),
                (OperationState::PowerDown, _) => return Err(Error::PoweredDown),
            }
        }
        Ok(())
    }

    #[allow(unused_variables)]
    async fn connect(&mut self, profile_id: ProfileId, context_id: ContextId) -> Result<(), Error> {
        info!("🔧 NetDevice::connect() - Starting data connection setup");
        debug!(
            "NetDevice::connect() - Profile ID: {:?}, Context ID: {:?}",
            profile_id, context_id
        );

        // This step _shouldn't_ be necessary.  However, for reasons I don't
        // understand, SARA-R4 can be registered but not attached (i.e. AT+CGATT
        // returns 0) on both RATs (unh?).  Phil Ware, who knows about these
        // things, always goes through (a) register, (b) wait for AT+CGATT to
        // return 1 and then (c) check that a context is active with AT+CGACT or
        // using AT+UPSD (even for EUTRAN). Since this sequence works for both
        // RANs, it is best to be consistent.
        info!("NetDevice::connect() - Waiting for network attachment (CGATT)");
        let mut attached = false;
        for attempt in 0..10 {
            debug!(
                "NetDevice::connect() - Network attachment attempt {}/{}",
                attempt + 1,
                10
            );
            match self.is_network_attached().await {
                Ok(true) => {
                    info!(
                        "NetDevice::connect() - Network successfully attached on attempt {}",
                        attempt + 1
                    );
                    attached = true;
                    break;
                }
                Ok(false) => {
                    warn!(
                        "NetDevice::connect() - Network not attached yet (attempt {})",
                        attempt + 1
                    );
                }
                Err(e) => {
                    warn!("NetDevice::connect() - Error checking attachment status on attempt {}: {:?}", attempt + 1, e);
                }
            }

            debug!("NetDevice::connect() - Waiting 1 second before next attachment check");
            Timer::after_secs(2).await;
        }

        if !attached {
            error!("NetDevice::connect() - Failed to attach to network after 10 attempts!");
            return Err(Error::AttachTimeout);
        }

        info!("NetDevice::connect() - Network attached, now activating context");

        // Activate the context
        #[cfg(feature = "use-upsd-context-activation")]
        {
            let apn_info = self.ch.get_apn_config();
            info!("NetDevice::connect() - Using UPSD context activation");
            match self.activate_context_upsd(profile_id, apn_info).await {
                Ok(_) => info!("NetDevice::connect() - Successfully activated context via UPSD"),
                Err(e) => {
                    error!(
                        "NetDevice::connect() - Failed to activate context via UPSD: {:?}",
                        e
                    );
                    return Err(e);
                }
            }
        }
        #[cfg(not(feature = "use-upsd-context-activation"))]
        {
            info!("NetDevice::connect() - Using 3GPP context activation");
            match self.activate_context(context_id, profile_id).await {
                Ok(_) => info!("NetDevice::connect() - Successfully activated context via 3GPP"),
                Err(e) => {
                    error!(
                        "NetDevice::connect() - Failed to activate context via 3GPP: {:?}",
                        e
                    );
                    return Err(e);
                }
            }
        }

        info!("✅ NetDevice::connect() - Data connection setup completed successfully");
        Ok(())
    }

    // Make sure we are attached to the cellular network.
    async fn is_network_attached(&mut self) -> Result<bool, Error> {
        debug!("NetDevice::is_network_attached() - Checking GPRS attachment status");
        // Check for AT+CGATT to return 1
        match self.at_client.send(&GetGPRSAttached).await {
            Ok(GPRSAttached { state }) => {
                let attached = state == GPRSAttachedState::Attached;
                debug!(
                    "NetDevice::is_network_attached() - GPRS state: {:?}, attached: {}",
                    state, attached
                );
                Ok(attached)
            }
            Err(e) => {
                error!(
                    "NetDevice::is_network_attached() - Failed to get GPRS attachment status: {:?}",
                    e
                );
                Err(e.into())
            }
        }
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
                .send(&SetPacketSwitchedConfig {
                    profile_id,
                    param: psn::types::PacketSwitchedParam::APN(
                        String::<99>::try_from(name).unwrap(),
                    ),
                })
                .await?;

            // Set up the user name
            if let Some(user_name) = username {
                self.at_client
                    .send(&SetPacketSwitchedConfig {
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
                    .send(&SetPacketSwitchedConfig {
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
            .send(&SetPacketSwitchedConfig {
                profile_id,
                param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
            })
            .await?;

        // Automatic authentication protocol selection
        self.at_client
            .send(&SetPacketSwitchedConfig {
                profile_id,
                param: psn::types::PacketSwitchedParam::Authentication(AuthenticationType::Auto),
            })
            .await?;

        self.at_client
            .send(&SetPacketSwitchedAction {
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
        #[allow(unused_variables)] profile_id: ProfileId,
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
                    use crate::command::psn::{
                        types::{
                            PacketSwitchedAction as PSAction, PacketSwitchedParam, ProtocolType,
                        },
                        SetPacketSwitchedAction, SetPacketSwitchedConfig,
                    };

                    self.at_client
                        .send(&SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::ProtocolType(ProtocolType::IPv4),
                        })
                        .await?;

                    self.at_client
                        .send(&SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::MapProfile(cid),
                        })
                        .await?;

                    // SARA-R5 pattern: the context also has to be
                    // activated and we're not actually done
                    // until the +UUPSDA URC comes back,
                    #[cfg(feature = "sara-r5")]
                    self.at_client
                        .send(&SetPacketSwitchedAction {
                            profile_id,
                            action: PSAction::Activate,
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
