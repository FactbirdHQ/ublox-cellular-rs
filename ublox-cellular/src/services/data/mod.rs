pub mod apn;
pub mod dns;
pub mod error;
pub mod socket;
pub mod ssl;
mod tcp_stack;
mod udp_stack;

mod hex;

use crate::{
    client::Device,
    command::mobile_control::types::Functionality,
    command::mobile_control::SetModuleFunctionality,
    command::network_service::types::RatAct,
    command::psn::types::GPRSAttachedState,
    command::psn::types::PDPContextStatus,
    command::psn::types::PacketSwitchedParam,
    command::psn::GetGPRSAttached,
    command::psn::GetPDPContextState,
    command::psn::SetGPRSAttached,
    command::psn::SetPDPContextDefinition,
    command::psn::SetPDPContextState,
    command::psn::SetPacketSwitchedAction,
    command::psn::SetPacketSwitchedConfig,
    command::{
        ip_transport_layer::{
            self,
            responses::{SocketData, UDPSocketData},
            ReadSocketData, ReadUDPSocketData,
        },
        psn, Urc,
    },
    error::Error as DeviceError,
    network::{ContextId, Error as NetworkError, Network},
    state::Event,
    ProfileId,
};
use apn::{APNInfo, Apn};
use atat::{typenum::Unsigned, AtatClient};
use core::cell::RefCell;
use embedded_hal::{
    blocking::delay::DelayMs,
    digital::{InputPin, OutputPin},
    timer::CountDown,
};
pub use error::Error;
use heapless::{ArrayLength, Bucket, Pos};
use psn::{
    types::{AuthenticationType, PacketSwitchedAction},
    SetAuthParameters,
};
use socket::{Error as SocketError, Socket, SocketRef, SocketSet, SocketType};

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
    N: ArrayLength<Option<Socket<L>>> + ArrayLength<Bucket<u8, usize>> + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    pub fn data_service<'a>(
        &'a mut self,
        cid: ContextId,
        apn_info: &APNInfo,
    ) -> nb::Result<DataService<'a, C, N, L>, DeviceError> {
        // Spin [`Device`], handling [`Network`] related URC changes and propagting the FSM
        let connected = self.spin()?;

        if let Some(ref sockets) = self.sockets {
            match DataService::try_new(cid, apn_info, &self.network, sockets, connected) {
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
    Registering,
    Active,
}

pub struct DataService<'a, C, N, L>
where
    C: atat::AtatClient,
    N: 'static
        + ArrayLength<Option<Socket<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    network: &'a Network<C>,
    pub(crate) sockets: &'a RefCell<&'static mut SocketSet<N, L>>,
}

impl<'a, C, N, L> DataService<'a, C, N, L>
where
    C: atat::AtatClient,
    N: 'static
        + ArrayLength<Option<Socket<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    pub fn try_new(
        cid: ContextId,
        apn_info: &APNInfo,
        network: &'a Network<C>,
        sockets: &'a RefCell<&'static mut SocketSet<N, L>>,
        connected: bool,
    ) -> nb::Result<Self, Error> {
        let mut data_service = Self { network, sockets };

        // Handle [`DataService`] related URCs
        data_service.handle_urc()?;

        // Reset context state if data connection is lost
        if matches!(network.context_state.get(), ContextState::Active) && !connected {
            network.context_state.set(ContextState::Registering);
        }

        let state = network.context_state.get();
        if !connected || state != ContextState::Active {
            // Check if context is active, and create if not
            match data_service.define_context(state, cid, apn_info) {
                Ok(state) => {
                    network.context_state.set(state);
                    if state != ContextState::Active {
                        return Err(nb::Error::WouldBlock);
                    }
                }
                Err(e) => {
                    if let nb::Error::Other(Error::Network(NetworkError::ActivationFailed)) = e {
                        network.context_state.set(ContextState::Setup);
                    }
                    return Err(nb::Error::WouldBlock);
                }
            };
        }

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

    fn set_pdn_config(&self, cid: ContextId, apn_info: &APNInfo) -> Result<(), Error> {
        self.network.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::AirplaneMode,
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

    fn deactivate_pdn(&self, cid: ContextId) -> Result<(), Error> {
        let status = self.network.is_registered().map_err(Error::from)?;
        if let Ok(state) = self.network.send_internal(&GetPDPContextState, true) {
            if state.cid == cid && state.status == PDPContextStatus::Activated {
                if state.cid != ContextId(1) && matches!(status.rat, RatAct::Lte) {
                    defmt::info!(
                        "Default Bearer context {:?} Active. Not allowed to deactivate",
                        1
                    );
                } else if self
                    .network
                    .send_internal(
                        &SetPDPContextState {
                            status: PDPContextStatus::Deactivated,
                            cid: Some(cid),
                        },
                        true,
                    )
                    .is_err()
                {
                    defmt::error!("can't deactivate PDN!");
                    if matches!(status.rat, RatAct::Gsm | RatAct::GsmGprsEdge)
                        && self.network.send_internal(&GetGPRSAttached, true)?.state
                            == GPRSAttachedState::Attached
                    {
                        defmt::error!("Deactivate Packet switch");
                        self.network.send_internal(
                            &SetGPRSAttached {
                                state: GPRSAttachedState::Detached,
                            },
                            true,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn define_context(
        &mut self,
        state: ContextState,
        cid: ContextId,
        apn_info: &APNInfo,
    ) -> nb::Result<ContextState, Error> {
        match state {
            ContextState::Setup => {
                /* Setup PDN. */
                self.set_pdn_config(cid, apn_info).map_err(Error::from)?;

                /* Rescan network. */
                // self.network
                //     .send_internal(
                //         &SetModuleFunctionality {
                //             fun: Functionality::AirplaneMode,
                //             rst: None,
                //         },
                //         true,
                //     ).map_err(Error::from)?;

                // self.network
                //     .send_internal(
                //         &SetModuleFunctionality {
                //             fun: Functionality::Full,
                //             rst: None,
                //         },
                //         true,
                //     )
                //     .map_err(Error::from)?;

                Ok(ContextState::Registering)
            }
            ContextState::Registering => {
                /* check registration status. */
                let service_status = self.network.is_registered().map_err(Error::from)?;
                if service_status.ps_reg_status.is_registered() {
                    // Emit Event::Attached
                    self.network
                        .push_event(Event::Attached)
                        .map_err(Error::from)?;
                } else {
                    // FIXME: Try count here with some failure break?!
                    return Err(nb::Error::WouldBlock);
                }

                /* Activate PDN. */
                if self.activate_pdn(cid).is_err() {
                    defmt::warn!("Activate PDN failed. Deactivate the PDN and retry");
                    // Ignore any error here!
                    self.deactivate_pdn(cid).ok();
                    if self.activate_pdn(cid).is_err() {
                        defmt::error!("Activate PDN failed after retry");
                        return Err(nb::Error::Other(Error::Network(
                            NetworkError::ActivationFailed,
                        )));
                    }
                }

                Ok(ContextState::Active)
            }
            ContextState::Active => Ok(ContextState::Active),
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
