use core::{cmp::Ordering, future::poll_fn, marker::PhantomData, task::Poll};

use crate::{
    asynch::state::OperationState,
    command::{
        general::GetCIMI,
        mobile_control::{
            responses::ModuleFunctionality,
            types::{Functionality, PowerMode},
            GetModuleFunctionality, SetModuleFunctionality,
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
    config::CellularConfig,
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

        if mcc_mnc.is_some() {
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

    async fn prepare_connect(&mut self) -> Result<(), Error> {
        // CREG URC
        self.at_client
            .send(&SetNetworkRegistrationStatus {
                n: NetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await?;

        // CGREG URC
        self.at_client
            .send(&SetGPRSNetworkRegistrationStatus {
                n: GPRSNetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await?;

        // CEREG URC
        self.at_client
            .send(&SetEPSNetworkRegistrationStatus {
                n: EPSNetworkRegistrationUrcConfig::UrcEnabled,
            })
            .await?;

        for _ in 0..10 {
            if self.at_client.send(&GetCIMI).await.is_ok() {
                break;
            }

            Timer::after_secs(1).await;
        }

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

    // /// Reset the module by sending AT CFUN command
    async fn soft_reset(&mut self, sim_reset: bool) -> Result<(), Error> {
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

    /// Check if we are registered to a network technology (uses +CxREG family
    /// commands)
    async fn wait_network_registered(&mut self, timeout: Duration) -> Result<(), Error> {
        let state_runner = self.ch.clone();
        let update_fut = async {
            loop {
                self.update_registration().await;

                Timer::after_millis(300).await;
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
                    self.run_to_desired().await?;
                }
                Either::Second(false) => {
                    warn!("Lost network registration. Setting operating state back to initialized");
                    self.ch.set_operation_state(OperationState::Initialized);
                    self.run_to_desired().await?;
                }
                Either::Second(true) => {
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

            info!("State transition: {} -> {}", current_state, desired_state);

            match (current_state, desired_state.cmp(&current_state)) {
                (_, Ordering::Equal) => break,

                (OperationState::Initialized, Ordering::Greater) => {
                    self.register_network(None).await?;
                    self.wait_network_registered(Duration::from_secs(180))
                        .await?;
                    self.ch.set_operation_state(OperationState::Connected);
                }
                (OperationState::Connected, Ordering::Greater) => {
                    match self.connect(C::APN, C::PROFILE_ID, C::CONTEXT_ID).await {
                        Ok(_) => {
                            #[cfg(not(feature = "use-upsd-context-activation"))]
                            self.ch
                                .set_profile_state(crate::registration::ProfileState::ShouldBeUp);

                            self.ch.set_operation_state(OperationState::DataEstablished);
                            Timer::after_secs(1).await;
                        }
                        Err(err) => {
                            // Switch radio off after failure
                            let _ = self.radio_off().await;
                            return Err(err);
                        }
                    }
                }

                // TODO: do proper backwards "single stepping"
                (OperationState::Connected, Ordering::Less) => {
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
            Timer::after_secs(1).await;
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
