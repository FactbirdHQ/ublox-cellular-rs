pub mod apn;
pub mod dns;
pub mod error;
pub mod ssl;

#[cfg(feature = "socket-tcp")]
mod tcp_stack;

#[cfg(feature = "socket-udp")]
mod udp_stack;

mod hex;

use crate::{
    client::Device,
    command::mobile_control::types::{Functionality, ResetMode},
    command::mobile_control::SetModuleFunctionality,
    command::psn::types::PDPContextStatus,
    command::psn::types::PacketSwitchedParam,
    command::psn::SetPDPContextDefinition,
    command::psn::SetPDPContextState,
    command::psn::SetPacketSwitchedConfig,
    command::{
        error::UbloxError,
        ip_transport_layer::{
            responses::{SocketData, UDPSocketData},
            ReadSocketData, ReadUDPSocketData,
        },
        psn::{
            self,
            responses::{GPRSAttached, PacketSwitchedConfig, PacketSwitchedNetworkData},
            types::PacketSwitchedParamReq,
            GetPDPContextState, GetPacketSwitchedConfig, GetPacketSwitchedNetworkData,
        },
    },
    command::{psn::SetPacketSwitchedAction, Urc},
    error::Error as DeviceError,
    error::GenericError,
    network::{ContextId, Network},
    ProfileId,
};
use apn::{APNInfo, Apn};
use atat::{AtatClient, Clock};
use embedded_hal::digital::blocking::{InputPin, OutputPin};
use fugit::ExtU32;

pub use error::Error;
use psn::{
    types::{
        AuthenticationType, GPRSAttachedState, PacketSwitchedAction, PacketSwitchedNetworkDataParam,
    },
    GetGPRSAttached, SetAuthParameters,
};
use ublox_sockets::{Error as SocketError, SocketSet, SocketType};

#[cfg(feature = "upsd-context-activation")]
use embedded_nal::Ipv4Addr;

// NOTE: If these are changed, remember to change the corresponding `Bytes` len
// in commands for now.
pub const INGRESS_CHUNK_SIZE: usize = 256;
pub const EGRESS_CHUNK_SIZE: usize = 1024;

const PROFILE_ID: ProfileId = ProfileId(1);

#[cfg(not(feature = "upsd-context-activation"))]
const CONTEXT_ID: ContextId = ContextId(1);

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
    /// Define a PDP context
    #[cfg(not(feature = "upsd-context-activation"))]
    fn define_context(&mut self, cid: ContextId, apn_info: &APNInfo) -> Result<(), Error> {
        if self.network.context_state != ContextState::Setup {
            return Ok(());
        }

        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Minimum,
                rst: Some(ResetMode::DontReset),
            },
            true,
        )?;

        if let Apn::Given(apn) = apn_info.clone().apn {
            self.network.send_internal(
                &SetPDPContextDefinition {
                    cid,
                    pdp_type: "IP",
                    apn: apn.as_str(),
                },
                true,
            )?;
        }

        self.network.send_internal(
            &SetAuthParameters {
                cid,
                auth_type: AuthenticationType::Auto,
                username: &apn_info.clone().user_name.unwrap_or_default(),
                password: &apn_info.clone().password.unwrap_or_default(),
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

        self.network.context_state = ContextState::Activating;
        Ok(())
    }

    pub fn data_service<'a>(
        &'a mut self,
        apn_info: &APNInfo,
    ) -> nb::Result<DataService<'a, C, CLK, TIMER_HZ, N, L>, DeviceError> {
        // Spin [`Device`], handling [`Network`] related URC changes and propagting the FSM
        match self.spin() {
            // If we're not using AT+UPSD-based
            // context activation, set the context using
            // AT+CGDCONT and the authentication mode
            Err(nb::Error::WouldBlock) => {
                #[cfg(not(feature = "upsd-context-activation"))]
                self.define_context(CONTEXT_ID, apn_info)
                    .map_err(DeviceError::from)?;
                return Err(nb::Error::WouldBlock);
            }
            Ok(()) => {
                #[cfg(not(feature = "upsd-context-activation"))]
                self.define_context(CONTEXT_ID, apn_info)
                    .map_err(DeviceError::from)?;
            }
            Err(e) => return Err(e),
        }

        // At this point we WILL be registered on the network!

        match DataService::try_new(apn_info, &mut self.network, self.sockets.as_deref_mut()) {
            Ok(service) => Ok(service),
            Err(nb::Error::Other(e)) => Err(nb::Error::Other(e.into())),
            Err(nb::Error::WouldBlock) => Err(nb::Error::WouldBlock),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ContextState {
    Setup,
    Activating,
    Active,
}

pub struct DataService<'a, C, CLK, const TIMER_HZ: u32, const N: usize, const L: usize>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
{
    network: &'a mut Network<C, CLK, TIMER_HZ>,
    pub(crate) sockets: Option<&'a mut SocketSet<TIMER_HZ, N, L>>,
}

impl<'a, C, CLK, const TIMER_HZ: u32, const N: usize, const L: usize>
    DataService<'a, C, CLK, TIMER_HZ, N, L>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
{
    pub fn try_new(
        apn_info: &APNInfo,
        network: &'a mut Network<C, CLK, TIMER_HZ>,
        sockets: Option<&'a mut SocketSet<TIMER_HZ, N, L>>,
    ) -> nb::Result<Self, Error> {
        let mut data_service = Self { network, sockets };

        // Check if context is active, and create if not
        data_service.connect(apn_info)?;

        // At this point [`data_service`] will always have a valid and active data context!

        // Attempt to ingress data from every open socket, into it's
        // internal rx buffer
        if data_service.sockets.is_some() {
            data_service.socket_ingress_all()?;
        }

        Ok(data_service)
    }

    #[allow(unused_variables)]
    fn connect(&mut self, apn_info: &APNInfo) -> nb::Result<(), Error> {
        match self.network.context_state {
            ContextState::Active => return Ok(()),
            ContextState::Setup | ContextState::Activating => {}
        }
        // This step _shouldn't_ be necessary.  However,
        // for reasons I don't understand, SARA-R4 can be
        // registered but not attached (i.e. AT+CGATT
        // returns 0) on both RATs (unh?).  Phil Ware, who
        // knows about these things, always goes through
        // (a) register, (b) wait for AT+CGATT to return 1
        // and then (c) check that a context is active
        // with AT+CGACT or using AT+UPSD (even for EUTRAN).
        // Since this sequence works for both RANs, it is
        // best to be consistent.
        self.attach_network()?;

        // Activate the context
        #[cfg(feature = "upsd-context-activation")]
        self.activate_context_upsd(PROFILE_ID, apn_info)?;
        #[cfg(not(feature = "upsd-context-activation"))]
        self.activate_context(CONTEXT_ID, PROFILE_ID)?;

        Ok(())
    }

    // Make sure we are attached to the cellular network.
    fn attach_network(&mut self) -> nb::Result<(), Error> {
        // Wait for AT+CGATT to return 1
        for _ in 0..10 {
            let GPRSAttached { state } = self
                .network
                .send_internal(&GetGPRSAttached, true)
                .map_err(Error::from)?;

            if state == GPRSAttachedState::Attached {
                return Ok(());
            }

            self.network
                .status
                .timer
                .start(1.secs())
                .map_err(|_e| Error::Generic(GenericError::Clock))?;
            self.network
                .status
                .timer
                .wait()
                .map_err(|_e| Error::Generic(GenericError::Clock))?;
        }

        // self.network
        //     .send_internal(
        //         &SetGPRSAttached {
        //             state: GPRSAttachedState::Attached,
        //         },
        //         true,
        //     )
        //     .map_err(Error::from)?;

        Err(nb::Error::WouldBlock)
    }

    /// Activate context using AT+UPSD commands, required
    /// for SARA-G3 and SARA-U2 modules.
    #[cfg(feature = "upsd-context-activation")]
    fn activate_context_upsd(
        &mut self,
        profile_id: ProfileId,
        apn_info: &APNInfo,
    ) -> nb::Result<(), Error> {
        if self.network.context_state == ContextState::Active {
            return Ok(());
        }

        // Check the if the PSD profile is activated (param_tag = 1)
        let PacketSwitchedNetworkData { param_tag, .. } = self
            .network
            .send_internal(
                &GetPacketSwitchedNetworkData {
                    profile_id,
                    param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
                },
                true,
            )
            .map_err(Error::from)?;

        if param_tag == 0 {
            self.network.context_state = ContextState::Activating;

            // SARA-U2 pattern: everything is done through AT+UPSD
            // Set up the APN
            if let Apn::Given(apn) = apn_info.clone().apn {
                self.network
                    .send_internal(
                        &SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::APN(apn),
                        },
                        true,
                    )
                    .map_err(Error::from)?;
            }

            // Set up the user name
            if let Some(user_name) = apn_info.clone().user_name {
                self.network
                    .send_internal(
                        &SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::Username(user_name),
                        },
                        true,
                    )
                    .map_err(Error::from)?;
            }

            // Set up the password
            if let Some(password) = apn_info.clone().password {
                self.network
                    .send_internal(
                        &SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::Password(password),
                        },
                        true,
                    )
                    .map_err(Error::from)?;
            }

            // Set up the dynamic IP address assignment.
            self.network
                .send_internal(
                    &SetPacketSwitchedConfig {
                        profile_id,
                        param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
                    },
                    true,
                )
                .map_err(Error::from)?;

            // Automatic authentication protocol selection
            self.network
                .send_internal(
                    &SetPacketSwitchedConfig {
                        profile_id,
                        param: PacketSwitchedParam::Authentication(AuthenticationType::Auto),
                    },
                    true,
                )
                .map_err(Error::from)?;

            self.network
                .send_internal(
                    &SetPacketSwitchedAction {
                        profile_id,
                        action: PacketSwitchedAction::Activate,
                    },
                    true,
                )
                .map_err(Error::from)?;
        }

        self.network.context_state = ContextState::Active;
        Ok(())
    }

    /// Activate context using 3GPP commands, required
    /// for SARA-R4/R5 and TOBY modules.
    #[cfg(not(feature = "upsd-context-activation"))]
    fn activate_context(&mut self, cid: ContextId, profile_id: ProfileId) -> nb::Result<(), Error> {
        if self.network.context_state == ContextState::Active {
            return Ok(());
        }

        let context_states = self
            .network
            .send_internal(&GetPDPContextState, true)
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
            // Note: SARA-R4 only supports a single context at any
            // one time and so doesn't require/support AT+UPSD.
            #[cfg(feature = "sara_r4")]
            return Ok(());

            if let PacketSwitchedConfig {
                param: PacketSwitchedParam::MapProfile(context),
                ..
            } = self
                .network
                .send_internal(
                    &GetPacketSwitchedConfig {
                        profile_id,
                        param: PacketSwitchedParamReq::MapProfile,
                    },
                    true,
                )
                .map_err(Error::from)?
            {
                if context != cid {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id,
                                param: PacketSwitchedParam::MapProfile(cid),
                            },
                            true,
                        )
                        .map_err(Error::from)?;

                    self.network
                        .send_internal(
                            &GetPacketSwitchedNetworkData {
                                profile_id,
                                param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
                            },
                            true,
                        )
                        .map_err(Error::from)?;
                }
            }

            let PacketSwitchedNetworkData { param_tag, .. } = self
                .network
                .send_internal(
                    &GetPacketSwitchedNetworkData {
                        profile_id,
                        param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
                    },
                    true,
                )
                .map_err(Error::from)?;

            if param_tag == 0 {
                self.network
                    .send_internal(
                        &SetPacketSwitchedAction {
                            profile_id,
                            action: PacketSwitchedAction::Activate,
                        },
                        true,
                    )
                    .map_err(Error::from)?;
            }

            self.network.context_state = ContextState::Active;
            Ok(())
        } else {
            self.network
                .send_internal(
                    &SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: Some(cid),
                    },
                    true,
                )
                .map_err(Error::from)?;

            Err(nb::Error::WouldBlock)
        }
    }

    pub fn send_at<A, const LEN: usize>(&mut self, cmd: &A) -> Result<A::Response, Error>
    where
        A: atat::AtatCmd<LEN>,
        A::Error: Into<UbloxError>,
    {
        Ok(self.network.send_internal(cmd, true)?)
    }

    pub fn handle_urc<F: FnOnce(Urc) -> bool>(&mut self, f: F) -> Result<(), Error> {
        self.network.at_tx.handle_urc(f).map_err(Error::Network)
    }

    fn socket_ingress_all(&mut self) -> Result<(), Error> {
        if let Some(ref mut sockets) = self.sockets {
            let network = &mut self.network;
            sockets
                .iter_mut()
                .map(|(handle, mut socket)| {
                    let available_data = socket.available_data();

                    if available_data == 0 {
                        // Check for new socket data available at regular intervals, just in case a URC is missed
                        if socket.should_update_available_data(network.status.timer.now()) {
                            if let Ok(SocketData { length, .. }) = network.send_internal(
                                &ReadSocketData {
                                    socket: handle,
                                    length: 0,
                                },
                                false,
                            ) {
                                socket.set_available_data(length);
                            }
                        }

                        return Ok(());
                    }

                    if !socket.can_recv() {
                        return Err(Error::BufferFull);
                    }

                    // Request [`IngressChunkSize`] if it is available, otherwise request
                    // maximum available data
                    let wanted_len = core::cmp::min(available_data, INGRESS_CHUNK_SIZE);
                    // Check if socket.buffer has room for wanted_len, and ingress the smallest of the two
                    let requested_len = core::cmp::min(wanted_len, socket.rx_window());

                    let (socket_handle, mut data, len) = match socket.get_type() {
                        SocketType::Tcp => {
                            // Allow room for 2x length (Hex), and command overhead
                            let SocketData {
                                socket,
                                data,
                                length,
                            } = network.send_internal(
                                &ReadSocketData {
                                    socket: handle,
                                    length: requested_len,
                                },
                                false,
                            )?;

                            (socket, data, length)
                        }
                        SocketType::Udp => {
                            // Allow room for 2x length (Hex), and command overhead
                            let UDPSocketData {
                                socket,
                                data,
                                length,
                                ..
                            } = network.send_internal(
                                &ReadUDPSocketData {
                                    socket: handle,
                                    length: requested_len,
                                },
                                false,
                            )?;

                            (socket, data, length)
                        }
                        _ => return Err(Error::WrongSocketType),
                    };

                    if socket_handle != handle {
                        error!("WrongSocketType {:?} != {:?}", socket_handle, handle);
                        return Err(Error::WrongSocketType);
                    }

                    if len == 0 {
                        socket.set_available_data(0);
                    }

                    if let Some(ref mut data) = data {
                        let hex_mode = true;
                        // let hex_mode = self.config.hex_mode;
                        let data_len = if hex_mode { data.len() / 2 } else { data.len() };
                        if len > 0 && data_len != len {
                            error!("BadLength {} != {}, {}", len, data_len, data.as_str());
                            return Err(Error::BadLength);
                        }

                        let demangled = if hex_mode {
                            hex::from_hex(unsafe { data.as_bytes_mut() })
                                .map_err(|_| Error::InvalidHex)?
                        } else {
                            data.as_bytes()
                        };

                        let enqueued = socket.rx_enqueue_slice(demangled);
                        if enqueued != demangled.len() {
                            // This should never happen, due to the `requested_len` check above
                            error!(
                                "Failed to enqueue full slice of data! {} != {}",
                                enqueued,
                                demangled.len()
                            );
                        }
                    } else {
                        return Err(Error::Socket(SocketError::Exhausted));
                    }

                    Ok(())
                })
                .filter_map(Result::err)
                .for_each(|_e| {
                    // error!(
                    //     "Failed to ingress data for socket! {:?}",
                    //     Debug2Format(&e)
                    // )
                });
            Ok(())
        } else {
            return Err(Error::SocketMemory);
        }
    }
}
