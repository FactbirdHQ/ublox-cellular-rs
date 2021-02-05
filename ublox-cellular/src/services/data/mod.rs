pub mod apn;
pub mod dns;
pub mod error;
pub mod socket;
pub mod ssl;

#[cfg(feature = "socket-tcp")]
mod tcp_stack;

#[cfg(feature = "socket-udp")]
mod udp_stack;

mod hex;

use crate::{
    client::Device,
    command::mobile_control::types::Functionality,
    command::mobile_control::SetModuleFunctionality,
    command::psn::types::PDPContextStatus,
    command::psn::types::PacketSwitchedParam,
    command::psn::SetPDPContextDefinition,
    command::psn::SetPDPContextState,
    command::psn::SetPacketSwitchedAction,
    command::psn::SetPacketSwitchedConfig,
    command::{
        ip_transport_layer::{
            responses::{SocketData, UDPSocketData},
            ReadSocketData, ReadUDPSocketData,
        },
        psn::{
            self,
            responses::{GPRSAttached, PacketSwitchedNetworkData},
            GetPDPContextState, GetPacketSwitchedNetworkData, SetGPRSAttached,
        },
    },
    error::Error as DeviceError,
    network::{ContextId, Error as NetworkError, Network},
    ProfileId,
};
use apn::{APNInfo, Apn};
use atat::{typenum::Unsigned, AtatClient};
use core::{cell::RefCell, convert::TryInto};
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};
pub use error::Error;
use heapless::{ArrayLength, Bucket, Pos};
use psn::{
    types::{
        AuthenticationType, GPRSAttachedState, PacketSwitchedAction, PacketSwitchedNetworkDataParam,
    },
    GetGPRSAttached, SetAuthParameters,
};
use socket::{Error as SocketError, Socket, SocketRef, SocketSet, SocketType};

// NOTE: If these are changed, remember to change the corresponding `Bytes` len
// in commands for now.
pub type IngressChunkSize = heapless::consts::U256;
pub type EgressChunkSize = heapless::consts::U1024;

impl<C, CLK, N, L, RST, DTR, PWR, VINT> Device<C, CLK, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<Socket<L>>> + ArrayLength<Bucket<u8, usize>> + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn data_service<'a>(
        &'a mut self,
        apn_info: &APNInfo,
    ) -> nb::Result<DataService<'a, C, CLK, N, L>, DeviceError> {
        // Spin [`Device`], handling [`Network`] related URC changes and propagting the FSM
        self.spin()?;

        if let Some(ref sockets) = self.sockets {
            match DataService::try_new(apn_info, &self.network, sockets) {
                Ok(service) => Ok(service),
                Err(nb::Error::Other(e)) => Err(nb::Error::Other(e.into())),
                Err(nb::Error::WouldBlock) => Err(nb::Error::WouldBlock),
            }
        } else {
            Err(nb::Error::Other(DeviceError::DataService(
                Error::SocketMemory,
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ContextState {
    Setup,
    Activating,
    Active,
}

pub struct DataService<'a, C, CLK, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
    N: 'static
        + ArrayLength<Option<Socket<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    network: &'a Network<C, CLK>,
    pub(crate) sockets: &'a RefCell<&'static mut SocketSet<N, L>>,
}

impl<'a, C, CLK, N, L> DataService<'a, C, CLK, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
    N: 'static
        + ArrayLength<Option<Socket<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    pub fn try_new(
        apn_info: &APNInfo,
        network: &'a Network<C, CLK>,
        sockets: &'a RefCell<&'static mut SocketSet<N, L>>,
    ) -> nb::Result<Self, Error> {
        let mut data_service = Self { network, sockets };

        // Handle [`DataService`] related URCs
        // data_service.handle_urc()?;

        // Check if context is active, and create if not
        data_service.setup_internal_context(apn_info)?;

        // At this point [`data_service`] will always have a valid and active data context!

        // Attempt to ingress data from every open socket, into it's
        // internal rx buffer
        data_service
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?
            .iter_mut()
            .try_for_each(|(_, socket)| data_service.socket_ingress(socket))?;

        Ok(data_service)
    }

    #[allow(dead_code)]
    fn set_pdn_config(&self, cid: ContextId, apn_info: &APNInfo) -> Result<(), Error> {
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::Minimum,
                rst: None,
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
                rst: None,
            },
            true,
        )?;

        Ok(())
    }

    #[allow(dead_code)]
    fn activate_pdn(&self, cid: ContextId) -> Result<(), Error> {
        if let Ok(state) = self.network.send_internal(&GetPDPContextState, true) {
            if state.cid == cid && state.status != PDPContextStatus::Activated {
                self.network.send_internal(
                    &SetPDPContextState {
                        status: PDPContextStatus::Activated,
                        cid: Some(cid),
                    },
                    true,
                )?;
            }
        } else {
            self.network.send_internal(
                &SetPDPContextState {
                    status: PDPContextStatus::Activated,
                    cid: Some(cid),
                },
                true,
            )?;
        }

        // TODO: Sometimes we get InvalidResponse on this?!
        self.network.send_internal(
            &SetPacketSwitchedConfig {
                profile_id: ProfileId(0),
                param: PacketSwitchedParam::MapProfile(cid),
            },
            true,
        )?;

        self.network.send_internal(
            &SetPacketSwitchedAction {
                profile_id: ProfileId(0),
                action: PacketSwitchedAction::Activate,
            },
            true,
        )?;

        Ok(())
    }

    // fn deactivate_pdn(&self, _cid: ContextId) -> Result<(), Error> {
    // let status = self.network.is_registered().map_err(Error::from)?;
    // if let Ok(state) = self.network.send_internal(&GetPDPContextState, true) {
    //     if state.cid == cid && state.status == PDPContextStatus::Activated {
    //         if state.cid != ContextId(1) && matches!(status.rat, RatAct::Lte) {
    //             defmt::info!(
    //                 "Default Bearer context {:?} Active. Not allowed to deactivate",
    //                 1
    //             );
    //         } else if self
    //             .network
    //             .send_internal(
    //                 &SetPDPContextState {
    //                     status: PDPContextStatus::Deactivated,
    //                     cid: Some(cid),
    //                 },
    //                 true,
    //             )
    //             .is_err()
    //         {
    //             defmt::error!("can't deactivate PDN!");
    //             if matches!(status.rat, RatAct::Gsm | RatAct::GsmGprsEdge)
    //                 && self.network.send_internal(&GetGPRSAttached, true)?.state
    //                     == GPRSAttachedState::Attached
    //             {
    //                 defmt::error!("Deactivate Packet switch");
    //                 self.network.send_internal(
    //                     &SetGPRSAttached {
    //                         state: GPRSAttachedState::Detached,
    //                     },
    //                     true,
    //                 )?;
    //             }
    //         }
    //     }
    // }
    //     Ok(())
    // }

    fn setup_internal_context(&mut self, apn_info: &APNInfo) -> nb::Result<(), Error> {
        if self.network.context_state.get() == ContextState::Active {
            return Ok(());
        }

        // Can be useful for debugging
        let force = false;

        let GPRSAttached { state } = self
            .network
            .send_internal(&GetGPRSAttached, true)
            .map_err(Error::from)?;

        if state == GPRSAttachedState::Detached {
            self.network
                .send_internal(
                    &SetGPRSAttached {
                        state: GPRSAttachedState::Attached,
                    },
                    true,
                )
                .map_err(Error::from)?;
        }

        // Check the if the PSD profile is activated (param_tag = 1)
        let PacketSwitchedNetworkData { param_tag, .. } = self
            .network
            .send_internal(
                &GetPacketSwitchedNetworkData {
                    profile_id: ProfileId(0),
                    param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
                },
                true,
            )
            .map_err(Error::from)?;

        match param_tag {
            0 => {
                self.network.context_state.set(ContextState::Activating);

                // Set up the dynamic IP address assignment.
                // self.network
                //     .send_internal(
                //         &SetPacketSwitchedConfig {
                //             profile_id: ProfileId(0),
                //             param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
                //         },
                //         true,
                //     )
                //     .map_err(Error::from)?;

                if let Apn::Given(apn) = apn_info.clone().apn {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id: ProfileId(0),
                                param: PacketSwitchedParam::APN(apn),
                            },
                            true,
                        )
                        .map_err(Error::from)?;
                }

                if let Some(user_name) = apn_info.clone().user_name {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id: ProfileId(0),
                                param: PacketSwitchedParam::Username(user_name),
                            },
                            true,
                        )
                        .map_err(Error::from)?;
                }

                if let Some(password) = apn_info.clone().password {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id: ProfileId(0),
                                param: PacketSwitchedParam::Password(password),
                            },
                            true,
                        )
                        .map_err(Error::from)?;
                }

                // self.network
                //     .send_internal(
                //         &SetPacketSwitchedConfig {
                //             profile_id: ProfileId(0),
                //             param: PacketSwitchedParam::Authentication(AuthenticationType::None),
                //         },
                //         true,
                //     )
                //     .map_err(Error::from)?;

                self.network
                    .send_internal(
                        &SetPacketSwitchedAction {
                            profile_id: ProfileId(0),
                            action: PacketSwitchedAction::Activate,
                        },
                        true,
                    )
                    .map_err(Error::from)?;

                self.network.context_state.set(ContextState::Active);
                Ok(())
            }
            1 => {
                if force {
                    // deactivate the PSD profile if it is already activated
                    self.network
                        .send_internal(
                            &SetPacketSwitchedAction {
                                profile_id: ProfileId(0),
                                action: PacketSwitchedAction::Deactivate,
                            },
                            true,
                        )
                        .map_err(Error::from)?;
                    Err(nb::Error::WouldBlock)
                } else {
                    self.network.context_state.set(ContextState::Active);
                    Ok(())
                }
            }
            _ => Err(nb::Error::Other(Error::Generic(
                crate::error::GenericError::Unsupported,
            ))),
        }
    }

    /// This is setting up an external PDP context, setup_internal_context() creates an internal one
    /// which is ultimately the one that's used by the system. So no need for this.
    #[allow(dead_code)]
    fn setup_external_context(&mut self, apn_info: &APNInfo) -> nb::Result<(), Error> {
        // Default context ID!
        let cid = ContextId(1);
        match self.network.context_state.get() {
            ContextState::Setup => {
                /* Setup PDN. */
                self.set_pdn_config(cid, apn_info).map_err(Error::from)?;
                self.network.context_state.set(ContextState::Activating);
                Err(nb::Error::WouldBlock)
            }
            ContextState::Activating => {
                /* Activate PDN. */
                self.activate_pdn(cid).map_err(|_| {
                    self.network.context_state.set(ContextState::Setup);
                    nb::Error::Other(Error::Network(NetworkError::ActivationFailed))
                })?;

                // TODO: Test this part! and rework based on findings!
                // if self.activate_pdn(cid).is_err() {
                //     // defmt::warn!("Activate PDN failed. Deactivate the PDN and retry");
                //     // self.deactivate_pdn(cid).ok();
                //     if self.activate_pdn(cid).is_err() {
                //         defmt::error!("Activate PDN failed after retry");
                //         return Err(nb::Error::Other(Error::Network(
                //             NetworkError::ActivationFailed,
                //         )));
                //     }
                // }

                self.network.context_state.set(ContextState::Active);
                Err(nb::Error::WouldBlock)
            }
            ContextState::Active => Ok(()),
        }
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        Ok(self.network.send_internal(cmd, true)?)
    }

    pub(crate) fn socket_ingress(&self, mut socket: SocketRef<Socket<L>>) -> Result<(), Error> {
        let handle = socket.handle();
        let available_data = socket.available_data();

        if available_data == 0 {
            return Ok(());
        }

        if !socket.can_recv() {
            return Err(Error::BufferFull);
        }

        // Request [`IngressChunkSize`] if it is available, otherwise request
        // maximum available data
        let wanted_len = core::cmp::min(available_data, IngressChunkSize::to_usize());
        // Check if socket.buffer has room for wanted_len, and ingress the smallest of the two
        let requested_len = core::cmp::min(wanted_len, socket.rx_window());

        let (socket_handle, mut data, len) = match socket.get_type() {
            SocketType::Tcp => {
                // Allow room for 2x length (Hex), and command overhead
                let SocketData {
                    socket,
                    data,
                    length,
                } = self.network.send_internal(
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
                } = self.network.send_internal(
                    &ReadUDPSocketData {
                        socket: handle,
                        length: requested_len,
                    },
                    false,
                )?;

                (socket, data, length)
            }
        };

        if socket_handle != handle {
            defmt::error!("WrongSocketType {:?} != {:?}", socket_handle, handle);
            return Err(Error::WrongSocketType);
        }

        if len == 0 {
            socket.set_available_data(0);
        }

        if let Some(ref mut data) = data {
            let hex_mode = true;
            // let hex_mode = self.config.try_borrow()?.hex_mode;
            let data_len = if hex_mode { data.len() / 2 } else { data.len() };
            if len > 0 && data_len != len {
                defmt::error!(
                    "BadLength {:?} != {:?}, {:str}",
                    len,
                    data_len,
                    data.as_str()
                );
                return Err(Error::BadLength);
            }

            let demangled = if hex_mode {
                hex::from_hex(unsafe { data.as_bytes_mut() }).map_err(|_| Error::InvalidHex)?
            } else {
                data.as_bytes()
            };

            socket.rx_enqueue_slice(demangled);

            Ok(())
        } else {
            Err(Error::Socket(SocketError::Exhausted))
        }
    }
}
