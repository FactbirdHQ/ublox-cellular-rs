pub mod apn;
pub mod dns;
pub mod error;
pub mod socket;
pub mod ssl;
mod tcp_stack;
// mod udp_stack;

mod hex;

use crate::{
    client::Device,
    command::psn::types::PDPContextStatus,
    command::psn::SetPDPContextDefinition,
    command::psn::SetPDPContextState,
    command::{
        general::{responses::CIMI, GetCIMI},
        ip_transport_layer::{
            self,
            responses::{SocketData, UDPSocketData},
            ReadSocketData, ReadUDPSocketData,
        },
        psn, Urc,
    },
    error::Error as DeviceError,
    network::{ContextId, Network, ProfileId, ProfileState},
    state::Event,
};
use apn::{APNInfo, Apn};
use atat::{typenum::Unsigned, AtatClient};
use core::cell::RefCell;
use embedded_hal::{
    blocking::delay::DelayMs,
    digital::{InputPin, OutputPin},
    timer::CountDown,
};
use embedded_nal::{IpAddr, Ipv4Addr};
pub use error::Error;
use heapless::{ArrayLength, Bucket, Pos, String};
use psn::{
    responses::PacketSwitchedNetworkData,
    types::{
        AuthenticationType, PacketSwitchedAction, PacketSwitchedNetworkDataParam,
        PacketSwitchedParam,
    },
    GetPacketSwitchedNetworkData, SetAuthParameters, SetPacketSwitchedAction,
    SetPacketSwitchedConfig,
};
use socket::{Error as SocketError, Socket, SocketRef, SocketSet, SocketSetItem, SocketType};

// NOTE: If these are changed, remember to change the corresponding `Bytes` len
// in commands for now.
pub type IngressChunkSize = heapless::consts::U256;
pub type EgressChunkSize = heapless::consts::U1024;

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
    pub fn data_service<'a>(
        &'a mut self,
        profile_id: ProfileId,
        cid: ContextId,
        apn_info: &APNInfo,
    ) -> nb::Result<DataService<'a, C, N, L>, DeviceError> {
        // Spin [`Device`], handling [`Network`] related URC changes and propagting the FSM
        self.spin()?;

        if let Some(ref sockets) = self.sockets {
            match DataService::try_new(profile_id, cid, apn_info, &self.network, sockets) {
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

// pub trait Tls: TcpStack {
//     type TlsConnector;
//
//     fn connect_tls(&self, connector: Self::TlsConnector, socket: <Self as TcpStack>::TcpSocket);
// }

// impl Tls for DataService {
//     type TlsConnector = SecurityProfileId;
//
//     fn connect_tls(&self, connector: Self::TlsConnector, socket: <Self as TcpStack>::TcpSocket) {
//         self.network.send_internal(
//             &SetSocketSslState {
//                 socket,
//                 ssl_tls_status: SslTlsStatus::Enabled(connector),
//             },
//             true,
//         )?;
//
//         TcpStack::Connect(self, socket)
//     }
// }

// impl core::convert::TryFrom<TlsConnectorBuilder<Device>> for SecurityProfileId {
//     type Error;
//
//     fn try_from(builder: TlsConnectorBuilder<Device>) -> Result<Self, Self::Error> {
//         if let Some(cert) = builder.cert {
//             builder.ctx.send_at(SetCertificate { cert })?;
//         }
//
//         let sec_id = 0;
//
//         self.network.send_internal(
//             &SecurityProfileManager {
//                 profile_id: sec_id,
//                 operation: Some(SecurityProfileOperation::CertificateValidationLevel(
//                     CertificateValidationLevel::RootCertValidationWithValidityDate,
//                 )),
//             },
//             true,
//         )?;
//
//         self.network.send_internal(
//             &SecurityProfileManager {
//                 profile_id: sec_id,
//                 operation: Some(SecurityProfileOperation::CipherSuite(0)),
//             },
//             true,
//         )?;
//
//         self.network.send_internal(
//             &SecurityProfileManager {
//                 profile_id: sec_id,
//                 operation: Some(SecurityProfileOperation::ExpectedServerHostname(
//                     builder.host_name,
//                 )),
//             },
//             true,
//         )?;
//
//         Ok(SecurityProfileId::new(sec_id))
//     }
// }

pub struct DataService<'a, C, N, L>
where
    C: atat::AtatClient,
    N: 'static
        + ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    network: &'a Network<C>,
    sockets: &'a RefCell<&'static mut SocketSet<N, L>>,
}

impl<'a, C, N, L> DataService<'a, C, N, L>
where
    C: atat::AtatClient,
    N: 'static
        + ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    pub fn try_new(
        profile_id: ProfileId,
        cid: ContextId,
        apn_info: &APNInfo,
        network: &'a Network<C>,
        sockets: &'a RefCell<&'static mut SocketSet<N, L>>,
    ) -> nb::Result<Self, Error> {
        let data_service = Self { network, sockets };

        // Handle [`DataService`] related URCs
        data_service.handle_urc()?;

        // Check if context is active, and create if not
        data_service.define_context(profile_id, cid, apn_info)?;

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

    fn define_context(
        &self,
        profile_id: ProfileId,
        cid: ContextId,
        apn_info: &APNInfo,
    ) -> nb::Result<(), Error> {
        // Check if profile is inactive
        if !self.is_profile_active(profile_id)? {
            match self
                .network
                .get_profile_state(profile_id)
                .map_err(|e| nb::Error::Other(e.into()))?
            {
                ProfileState::Deactivated | ProfileState::Unknown => {
                    defmt::debug!("[ProfileState] Deactivated | Unknown");

                    // Prune all sockets upon a clean network connection
                    self.sockets
                        .try_borrow_mut()
                        .map_err(|e| nb::Error::Other(e.into()))?
                        .prune();

                    match &apn_info.apn {
                        Apn::Given(_) => {
                            self.network
                                .set_profile_state(
                                    profile_id,
                                    ProfileState::Activating(cid, apn_info.clone()),
                                )
                                .map_err(|e| nb::Error::Other(e.into()))?;
                        }
                        Apn::Automatic => {
                            let CIMI { imsi: _ } = self
                                .network
                                .send_internal(&GetCIMI, true)
                                .map_err(|e| nb::Error::Other(e.into()))?;
                            // Lookup APN in DB for `imsi`
                            let apn_info_from_db = APNInfo {
                                apn: Apn::Given(String::new()),
                                ..apn_info.clone()
                            };
                            self.network
                                .set_profile_state(
                                    profile_id,
                                    ProfileState::Activating(cid, apn_info_from_db),
                                )
                                .map_err(|e| nb::Error::Other(e.into()))?;
                        }
                    }
                    defmt::debug!("[ProfileState] Attempting activation!");

                    match self.activate_profile(profile_id) {
                        Err(nb::Error::Other(_e)) => {
                            // Find next APN to try!
                            // Lookup APN in DB for `imsi`
                            // let apn_info_from_db = APNInfo {
                            //     apn: Apn::Given(String::new()),
                            //     ..APNInfo::default()
                            // };
                            // self.network.set_profile_state(
                            //     profile_id,
                            //     ProfileState::Activating(apn_info_from_db),
                            // )?;
                            // Err(nb::Error::WouldBlock)
                            self.network
                                .set_profile_state(profile_id, ProfileState::Deactivated)
                                .map_err(|e| nb::Error::Other(e.into()))?;

                            self.network.push_event(Event::Disconnected(Some(cid))).ok();

                            Err(nb::Error::Other(Error::InvalidApn))
                        }
                        Ok(()) => {
                            // Ok(()) indicates that an LTE context is already active
                            defmt::debug!("[ProfileState] Shortcut LTE context!");

                            self.network
                                .finish_activating_profile_state(None)
                                .map_err(|e| nb::Error::Other(e.into()))?;
                            Ok(())
                        }
                        Err(nb::Error::WouldBlock) => Err(nb::Error::WouldBlock),
                    }
                }
                ProfileState::Activating(_, _) => Err(nb::Error::WouldBlock),
                ProfileState::Active(c, _ip_addr) if c != cid => {
                    defmt::debug!("[ProfileState] Active(c != cid)");

                    // Profile is already active, with a different ContextId. Return Error
                    Err(nb::Error::Other(Error::_Unknown))
                }
                ProfileState::Active(_, _) => {
                    defmt::error!("[ProfileState] Active(c == cid). SHOULD NEVER HAPPEN!");
                    Ok(())
                }
            }
        } else {
            // If the profile is already active, we're good
            Ok(())
        }
    }

    fn is_profile_active(&self, profile_id: ProfileId) -> Result<bool, Error> {
        if let ProfileState::Active(_, _) = self.network.get_profile_state(profile_id)? {
            return Ok(true);
        }

        if let Ok(PacketSwitchedNetworkData { param_tag, .. }) = self.network.send_internal(
            &GetPacketSwitchedNetworkData {
                profile_id,
                param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
            },
            true,
        ) {
            Ok(param_tag == 1)
        } else {
            // Just return false in case of errors or timeouts
            Ok(false)
        }
    }

    fn activate_profile(&self, profile_id: ProfileId) -> nb::Result<(), Error> {
        if let ProfileState::Activating(cid, apn) = self
            .network
            .get_profile_state(profile_id)
            .map_err(|e| nb::Error::Other(e.into()))?
        {
            // FIXME: Figure out which of these two approaches to use when, and why?
            if false {
                if let Apn::Given(apn) = apn.apn {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id,
                                param: PacketSwitchedParam::APN(apn),
                            },
                            true,
                        )
                        .map_err(|e| nb::Error::Other(e.into()))?;
                }
                if let Some(user_name) = apn.user_name {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id,
                                param: PacketSwitchedParam::Username(user_name),
                            },
                            true,
                        )
                        .map_err(|e| nb::Error::Other(e.into()))?;
                }

                if let Some(password) = apn.password {
                    self.network
                        .send_internal(
                            &SetPacketSwitchedConfig {
                                profile_id,
                                param: PacketSwitchedParam::Password(password),
                            },
                            true,
                        )
                        .map_err(|e| nb::Error::Other(e.into()))?;
                }

                self.network
                    .send_internal(
                        &SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::IPAddress(IpAddr::V4(
                                Ipv4Addr::unspecified(),
                            )),
                        },
                        true,
                    )
                    .map_err(|e| nb::Error::Other(e.into()))?;
            } else {
                if let Apn::Given(apn) = apn.apn {
                    self.network
                        .send_internal(
                            &SetPDPContextDefinition {
                                cid,
                                pdp_type: "IP",
                                apn: apn.as_str(),
                            },
                            true,
                        )
                        .map_err(|e| nb::Error::Other(e.into()))?;
                }

                self.network
                    .send_internal(
                        &SetAuthParameters {
                            cid,
                            auth_type: AuthenticationType::Auto,
                            username: &apn.user_name.unwrap_or_default(),
                            password: &apn.password.unwrap_or_default(),
                        },
                        true,
                    )
                    .map_err(|e| nb::Error::Other(e.into()))?;
                self.network
                    .send_internal(
                        &SetPacketSwitchedConfig {
                            profile_id,
                            param: PacketSwitchedParam::MapProfile(cid),
                        },
                        true,
                    )
                    .map_err(|e| nb::Error::Other(e.into()))?;

                // FIXME: https://github.com/BlackbirdHQ/atat/issues/63
                // let PDPContextState { status } =
                //     self.network.send_internal(&GetPDPContextState, true)?;

                // if status == PDPContextStatus::Deactivated {
                // If not active, help it on its way.
                self.network
                    .send_internal(
                        &SetPDPContextState {
                            status: PDPContextStatus::Activated,
                            cid: Some(cid),
                        },
                        true,
                    )
                    .map_err(|e| nb::Error::Other(e.into()))?;
                // }
            }

            self.network
                .send_internal(
                    &SetPacketSwitchedAction {
                        profile_id,
                        action: PacketSwitchedAction::Activate,
                    },
                    true,
                )
                .map_err(|e| nb::Error::Other(e.into()))?;

            Err(nb::Error::WouldBlock)
        } else {
            defmt::error!("ProfileState ERROR");
            Err(nb::Error::Other(Error::_Unknown))
        }
    }

    fn handle_urc(&self) -> Result<(), Error> {
        self.network
            .at_tx
            .handle_urc(|urc| {
                match urc {
                    Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket }) => {
                        defmt::info!("[URC] SocketClosed {:u8}", socket.0);
                        if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
                            sockets.remove(socket).ok();
                        }
                    }
                    Urc::SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable {
                        socket,
                        length,
                    })
                    | Urc::SocketDataAvailableUDP(ip_transport_layer::urc::SocketDataAvailable {
                        socket,
                        length,
                    }) => {
                        defmt::trace!(
                            "[Socket({:u8})] {:u16} bytes available",
                            socket.0,
                            length as u16
                        );
                        if let Ok(mut sockets) = self.sockets.try_borrow_mut() {
                            if let Some((_, mut sock)) =
                                sockets.iter_mut().find(|(handle, _)| *handle == socket)
                            {
                                sock.set_available_data(length);
                            }
                        } else {
                            defmt::warn!("[Socket({:u8})] Failed to borrow socketset!", socket.0);
                        }
                    }
                    _ => {
                        defmt::info!("[URC] (DataService) Unhandled URC");
                        return false;
                    }
                }
                true
            })
            .map_err(Error::Network)
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
