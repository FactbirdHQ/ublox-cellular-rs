use super::ssl::SecurityProfileId;
use super::DataService;
use super::EGRESS_CHUNK_SIZE;
use crate::command::ip_transport_layer::{
    types::{SocketProtocol, SslTlsStatus},
    CloseSocket, ConnectSocket, CreateSocket, PrepareWriteSocketDataBinary, SetSocketSslState,
    WriteSocketDataBinary,
};
use embedded_nal::{SocketAddr, TcpClientStack};
use ublox_sockets::{Error, SocketHandle, TcpSocket, TcpState};

impl<'a, C, CLK, const TIMER_HZ: u32, const N: usize, const L: usize> TcpClientStack
    for DataService<'a, C, CLK, TIMER_HZ, N, L>
where
    C: atat::blocking::AtatClient,
    CLK: fugit_timer::Timer<TIMER_HZ>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn socket(&mut self) -> Result<Self::TcpSocket, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            // Check if there are any unused sockets available
            if sockets.len() >= sockets.capacity() {
                let ts = self.network.status.timer.now();
                // Check if there are any sockets closed by remote, and close it
                // if it has exceeded its timeout, in order to recycle it.
                if !sockets.recycle(ts) {
                    return Err(Error::SocketSetFull);
                }
            }

            let socket_resp = self
                .network
                .send_internal(
                    &CreateSocket {
                        protocol: SocketProtocol::TCP,
                        local_port: None,
                    },
                    true,
                )
                .map_err(|_| Error::Unaddressable)?;

            Ok(sockets.add(TcpSocket::new(socket_resp.socket.0))?)
        } else {
            Err(Error::Illegal)
        }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: &mut Self::TcpSocket,
        remote: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            let mut tcp = sockets
                .get::<TcpSocket<TIMER_HZ, L>>(*socket)
                .map_err(Self::Error::from)?;

            if matches!(tcp.state(), TcpState::Created) {
                self.network
                    .send_internal(
                        &SetSocketSslState {
                            socket: *socket,
                            ssl_tls_status: SslTlsStatus::Enabled(SecurityProfileId(0)),
                        },
                        true,
                    )
                    .map_err(|_| nb::Error::Other(Error::Unaddressable))?;

                self.network
                    .send_internal(
                        &ConnectSocket {
                            socket: *socket,
                            remote_addr: remote.ip(),
                            remote_port: remote.port(),
                        },
                        false,
                    )
                    .map_err(|_| nb::Error::Other(Error::Unaddressable))?;

                tcp.set_state(TcpState::Connected(remote));
                Ok(())
            } else {
                error!(
                    "Cannot connect socket! Socket: {:?} is in state: {:?}",
                    socket,
                    tcp.state()
                );
                Err(Error::Illegal.into())
            }
        } else {
            Err(Error::Illegal.into())
        }
    }

    /// Check if this socket is still connected
    fn is_connected(&mut self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            Ok(sockets
                .get::<TcpSocket<TIMER_HZ, L>>(*socket)?
                .is_connected())
        } else {
            Err(Error::Illegal)
        }
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn send(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &[u8],
    ) -> nb::Result<usize, Self::Error> {
        if !self.is_connected(socket)? {
            return Err(Error::SocketClosed.into());
        }

        for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
            trace!("Sending: {} bytes", chunk.len());
            self.network
                .send_internal(
                    &PrepareWriteSocketDataBinary {
                        socket: *socket,
                        length: chunk.len(),
                    },
                    false,
                )
                .map_err(|_| nb::Error::Other(Error::Unaddressable))?;

            let response = self
                .network
                .send_internal(
                    &WriteSocketDataBinary {
                        data: atat::serde_bytes::Bytes::new(chunk),
                    },
                    false,
                )
                .map_err(|_| nb::Error::Other(Error::Unaddressable))?;

            if response.length != chunk.len() {
                return Err(Error::BadLength.into());
            }
            if &response.socket != socket {
                return Err(Error::InvalidSocket.into());
            }
        }

        Ok(buffer.len())
    }

    /// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
    /// data have been received and they have been placed in
    /// `&buffer[0..n]`, or an error.
    fn receive(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            let mut tcp = sockets
                .get::<TcpSocket<TIMER_HZ, L>>(*socket)
                .map_err(Self::Error::from)?;

            Ok(tcp.recv_slice(buffer).map_err(Self::Error::from)?)
        } else {
            Err(Error::Illegal.into())
        }
    }

    /// Close an existing TCP socket.
    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            self.network
                .send_internal(&CloseSocket { socket }, false)
                .ok();
            sockets.remove(socket)?;
            Ok(())
        } else {
            Err(Error::Illegal)
        }
    }
}
