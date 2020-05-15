use embedded_hal::digital::v2::OutputPin;
pub use embedded_nal::{Ipv4Addr, Mode, SocketAddr, SocketAddrV4};
use heapless::consts;

use crate::command::ip_transport_layer::{types::*, *};
use crate::error::Error;
use crate::modules::ssl::SSL;
use crate::GsmClient;
use typenum::marker_traits::Unsigned;

use crate::{
    command::ip_transport_layer::responses::SocketErrorResponse,
    hex,
    socket::{SocketHandle, SocketType},
};

#[cfg(feature = "socket-udp")]
use crate::socket::UdpSocket;
#[cfg(feature = "socket-udp")]
use embedded_nal::UdpStack;

#[cfg(feature = "socket-tcp")]
use crate::socket::{TcpSocket, TcpState};
#[cfg(feature = "socket-tcp")]
use embedded_nal::TcpStack;

pub type IngressChunkSize = consts::U256;
pub type EgressChunkSize = consts::U512;

impl<C, RST, DTR> GsmClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    /// Helper function to manage the internal poll counter, used to poll open
    /// sockets for incoming data, in case a `SocketDataAvailable` URC is missed
    /// once in a while, as the ublox module will never send the URC again, if
    /// the socket is not read.
    pub(crate) fn poll_cnt(&self, reset: bool) -> u16 {
        match self.poll_cnt.try_borrow_mut() {
            Ok(mut pc) => {
                if reset {
                    // Reset poll_cnt
                    *pc = 0;
                    0
                } else {
                    // Increment poll_cnt by one, and return the old value
                    *pc += 1;
                    *pc - 1
                }
            }
            Err(_) => 0,
        }
    }

    pub(crate) fn handle_socket_error<A: atat::AtatResp, F: Fn() -> Result<A, Error>>(
        &self,
        f: F,
        socket: Option<SocketHandle>,
        attempt: u8,
    ) -> Result<A, Error> {
        match f() {
            Ok(r) => Ok(r),
            Err(e @ Error::AT(atat::Error::Timeout)) => {
                if attempt < 3 {
                    #[cfg(feature = "logging")]
                    log::error!("[RETRY] Retrying! {:?}", attempt);
                    self.handle_socket_error(f, socket, attempt + 1)
                } else {
                    Err(e)
                }
            }
            Err(e @ Error::AT(atat::Error::InvalidResponse)) => {
                // let SocketErrorResponse { error } = self
                //     .send_internal(&GetSocketError, false)
                //     .unwrap_or_else(|_e| SocketErrorResponse { error: 110 });

                // if error != 0 {
                if let Some(handle) = socket {
                    let mut sockets = self.sockets.try_borrow_mut()?;
                    match sockets.socket_type(&handle) {
                        Some(SocketType::Tcp) => {
                            let mut tcp = sockets.get::<TcpSocket<_>>(&handle)?;
                            tcp.close();
                        }
                        Some(SocketType::Udp) => {
                            let mut udp = sockets.get::<UdpSocket<_>>(&handle)?;
                            udp.close();
                        }
                        _ => {}
                    }
                    sockets.remove(handle)?;
                }
                Err(e)
            }
            Err(e) => Err(e),
        }
    }

    pub(crate) fn socket_ingress(
        &self,
        socket: &SocketHandle,
        length: usize,
    ) -> Result<usize, Error> {
        if length == 0 {
            return Ok(0);
        }

        // Allow room for 2x length (Hex), and command overhead
        let chunk_size = core::cmp::min(length, IngressChunkSize::to_usize());
        let mut sockets = self.sockets.try_borrow_mut()?;

        // Reset poll_cnt
        self.poll_cnt(true);

        match sockets.socket_type(socket) {
            Some(SocketType::Tcp) => {
                // Handle tcp socket
                let mut tcp = sockets.get::<TcpSocket<_>>(socket)?;
                if !tcp.may_recv() {
                    return Err(Error::SocketClosed);
                }

                let mut socket_data = self.handle_socket_error(
                    || {
                        self.send_internal(
                            &ReadSocketData {
                                socket,
                                length: chunk_size,
                            },
                            false,
                        )
                    },
                    Some(socket.clone()),
                    0,
                )?;

                if &socket_data.socket != socket {
                    #[cfg(feature = "logging")]
                    log::error!("WrongSocketType {:?} != {:?}", socket_data.socket, socket);
                    return Err(Error::WrongSocketType);
                }

                if let Some(ref mut data) = socket_data.data {
                    if socket_data.length > 0 && data.len() / 2 != socket_data.length {
                        #[cfg(feature = "logging")]
                        log::error!(
                            "BadLength {:?} != {:?}, {:?}",
                            socket_data.length,
                            data.len() / 2,
                            data
                        );
                        return Err(Error::BadLength);
                    }

                    Ok(tcp.rx_enqueue_slice(
                        hex::from_hex(unsafe { data.as_bytes_mut() })
                            .map_err(|_| Error::InvalidHex)?,
                    ))
                } else {
                    Ok(0)
                }
            }
            Some(SocketType::Udp) => {
                // Handle udp socket
                let mut udp = sockets.get::<UdpSocket<_>>(socket)?;

                let mut socket_data = self.send_internal(
                    &ReadUDPSocketData {
                        socket,
                        length: chunk_size,
                    },
                    false,
                )?;

                if socket_data.length != chunk_size {
                    return Err(Error::BadLength);
                }

                if &socket_data.socket != socket {
                    return Err(Error::WrongSocketType);
                }

                if let Some(ref mut data) = socket_data.data {
                    Ok(udp.rx_enqueue_slice(
                        hex::from_hex(unsafe { data.as_bytes_mut() })
                            .map_err(|_| Error::InvalidHex)?,
                    ))
                } else {
                    Ok(0)
                }
            }
            _ => Err(Error::WrongSocketType),
        }
    }
}

#[cfg(feature = "socket-udp")]
impl<C, RST, DTR> UdpStack for GsmClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    /// Open a new UDP socket to the given address and port. UDP is connectionless,
    /// so unlike `TcpStack` no `connect()` is required.
    fn open(&self, remote: SocketAddr, _mode: Mode) -> Result<Self::UdpSocket, Self::Error> {
        let socket_resp = self.handle_socket_error(
            || {
                self.send_internal(
                    &CreateSocket {
                        protocol: SocketProtocol::UDP,
                        local_port: None,
                    },
                    false,
                )
            },
            None,
            0,
        )?;

        let mut socket = UdpSocket::new(socket_resp.socket.0);
        socket.bind(remote)?;

        Ok(self.sockets.try_borrow_mut()?.add(socket)?)
    }

    /// Send a datagram to the remote host.
    fn write(&self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let udp = sockets
            .get::<UdpSocket<_>>(&socket)
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        if !udp.is_open() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            // #[cfg(feature = "logging")]
            // log::debug!("Sending: {} bytes, {:?}", chunk.len(), chunk);
            self.handle_socket_error(
                || {
                    self.send_internal(
                        &PrepareUDPSendToDataBinary {
                            socket: socket.clone(),
                            remote_addr: udp.endpoint.ip(),
                            remote_port: udp.endpoint.port(),
                            length: chunk.len(),
                        },
                        false,
                    )
                },
                Some(socket.clone()),
                0,
            )?;

            let response = self.handle_socket_error(
                || {
                    self.send_internal(
                        &UDPSendToDataBinary {
                            data: serde_at::ser::Bytes(chunk),
                        },
                        false,
                    )
                },
                Some(socket.clone()),
                0,
            )?;

            if response.length != chunk.len() {
                return Err(nb::Error::Other(Error::BadLength));
            }
            if &response.socket != socket {
                return Err(nb::Error::Other(Error::WrongSocketType));
            }
        }

        return Ok(());
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn read(
        &self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        self.spin()?;

        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;
        let mut udp = sockets
            .get::<UdpSocket<_>>(&socket)
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;
        return udp
            .recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()));
    }

    /// Close an existing UDP socket.
    fn close(&self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        self.send_internal(&CloseSocket { socket }, false)?;

        let mut sockets = self.sockets.try_borrow_mut()?;
        let mut udp = sockets.get::<UdpSocket<_>>(&socket)?;
        udp.close();

        self.handle_socket_error(
            || self.send_internal(&CloseSocket { socket }, false),
            Some(socket),
            0,
        )?;

        sockets.remove(socket)?;

        Ok(())
    }
}

#[cfg(feature = "socket-tcp")]
impl<C, RST, DTR> TcpStack for GsmClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn open(&self, _mode: Mode) -> Result<Self::TcpSocket, Self::Error> {
        if self.get_state()? != crate::client::State::Attached {
            return Err(Error::Network);
        }

        let socket_resp = self.handle_socket_error(
            || {
                self.send_internal(
                    &CreateSocket {
                        protocol: SocketProtocol::TCP,
                        local_port: None,
                    },
                    false,
                )
            },
            None,
            0,
        )?;

        Ok(self
            .sockets
            .try_borrow_mut()?
            .add(TcpSocket::new(socket_resp.socket.0))?)
    }

    /// Connect to the given remote host and port.
    fn connect(
        &self,
        socket: Self::TcpSocket,
        remote: SocketAddr,
    ) -> Result<Self::TcpSocket, Self::Error> {
        if self.get_state()? != crate::client::State::Attached {
            return Err(Error::Network);
        }

        self.enable_ssl(socket, 0)?;

        self.handle_socket_error(
            || {
                self.send_internal(
                    &ConnectSocket {
                        socket,
                        remote_addr: remote.ip(),
                        remote_port: remote.port(),
                    },
                    false,
                )
            },
            Some(socket),
            0,
        )?;

        let mut sockets = self.sockets.try_borrow_mut()?;
        let mut tcp = sockets.get::<TcpSocket<_>>(&socket)?;
        tcp.set_state(TcpState::Established);
        Ok(tcp.handle())
    }

    /// Check if this socket is still connected
    fn is_connected(&self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        if self.get_state()? != crate::client::State::Attached {
            return Ok(false);
        }

        let mut sockets = self.sockets.try_borrow_mut()?;
        Ok(sockets.get::<TcpSocket<_>>(&socket)?.is_active())
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn write(&self, socket: &mut Self::TcpSocket, buffer: &[u8]) -> nb::Result<usize, Self::Error> {
        if !self.is_connected(&socket)? {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            // #[cfg(feature = "logging")]
            // log::debug!("Sending: {} bytes, {:?}", chunk.len(), chunk);
            self.handle_socket_error(
                || {
                    self.send_internal(
                        &PrepareWriteSocketDataBinary {
                            socket: socket.clone(),
                            length: chunk.len(),
                        },
                        false,
                    )
                },
                Some(socket.clone()),
                0,
            )?;

            let response = self.handle_socket_error(
                || {
                    self.send_internal(
                        &WriteSocketDataBinary {
                            data: serde_at::ser::Bytes(chunk),
                        },
                        false,
                    )
                },
                Some(socket.clone()),
                0,
            )?;

            if response.length != chunk.len() {
                return Err(nb::Error::Other(Error::BadLength));
            }
            if &response.socket != socket {
                return Err(nb::Error::Other(Error::WrongSocketType));
            }
        }

        return Ok(buffer.len());
    }

    /// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
    /// data have been received and they have been placed in
    /// `&buffer[0..n]`, or an error.
    fn read(
        &self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        self.spin()?;

        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let mut tcp = sockets
            .get::<TcpSocket<_>>(&socket)
            .map_err(|e| nb::Error::Other(e.into()))?;

        tcp.recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))
    }

    /// Close an existing TCP socket.
    fn close(&self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut()?;
        let mut tcp = sockets.get::<TcpSocket<_>>(&socket)?;
        tcp.close();

        sockets.remove(socket)?;

        self.send_internal(&CloseSocket { socket }, false)?;

        Ok(())
    }
}
