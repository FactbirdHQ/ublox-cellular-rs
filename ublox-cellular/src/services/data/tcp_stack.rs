use core::convert::TryInto;

use super::ssl::SecurityProfileId;
use super::DataService;
use super::{
    socket::{Error as SocketError, SocketHandle, TcpSocket, TcpState},
    Error, EGRESS_CHUNK_SIZE,
};
use crate::command::ip_transport_layer::{
    types::{SocketProtocol, SslTlsStatus},
    CloseSocket, ConnectSocket, CreateSocket, PrepareWriteSocketDataBinary, SetSocketSslState,
    WriteSocketDataBinary,
};
use embedded_nal::{SocketAddr, TcpClientStack};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};

impl<'a, C, CLK, const N: usize, const L: usize> TcpClientStack for DataService<'a, C, CLK, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn socket(&mut self) -> Result<Self::TcpSocket, Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut()?;

        // Check if there are any unused sockets available
        if sockets.len() >= sockets.capacity() {
            if let Ok(ts) = self.network.status.try_borrow_mut()?.timer.try_now() {
                // Check if there are any sockets closed by remote, and close it
                // if it has exceeded its timeout, in order to recycle it.
                if sockets.recycle(&ts) {
                    return Err(Error::Socket(SocketError::SocketSetFull));
                }
            } else {
                return Err(Error::Socket(SocketError::SocketSetFull));
            }
        }

        let socket_resp = self.network.send_internal(
            &CreateSocket {
                protocol: SocketProtocol::TCP,
                local_port: None,
            },
            true,
        )?;

        Ok(sockets.add(TcpSocket::new(socket_resp.socket.0))?)
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: &mut Self::TcpSocket,
        remote: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;
        let mut tcp = sockets
            .get::<TcpSocket<CLK, L>>(*socket)
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
                .map_err(Self::Error::from)?;

            self.network
                .send_internal(
                    &ConnectSocket {
                        socket: *socket,
                        remote_addr: remote.ip(),
                        remote_port: remote.port(),
                    },
                    false,
                )
                .map_err(Self::Error::from)?;

            tcp.set_state(TcpState::Connected);
            Ok(())
        } else {
            defmt::error!("Cannot connect socket!");
            Err(Error::Socket(SocketError::Illegal).into())
        }
    }

    /// Check if this socket is still connected
    fn is_connected(&mut self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut()?;
        Ok(sockets.get::<TcpSocket<CLK, L>>(*socket)?.is_connected())
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn send(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &[u8],
    ) -> nb::Result<usize, Self::Error> {
        if !self.is_connected(&socket)? {
            return Err(Error::SocketClosed.into());
        }

        for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
            defmt::trace!("Sending: {} bytes", chunk.len());
            self.network
                .send_internal(
                    &PrepareWriteSocketDataBinary {
                        socket: *socket,
                        length: chunk.len(),
                    },
                    false,
                )
                .map_err(Self::Error::from)?;

            let response = self
                .network
                .send_internal(
                    &WriteSocketDataBinary {
                        data: atat::serde_at::ser::Bytes(chunk),
                    },
                    false,
                )
                .map_err(Self::Error::from)?;

            if response.length != chunk.len() {
                return Err(Error::BadLength.into());
            }
            if &response.socket != socket {
                return Err(Error::WrongSocketType.into());
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
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut tcp = sockets
            .get::<TcpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?;

        Ok(tcp.recv_slice(buffer).map_err(Self::Error::from)?)
    }

    /// Close an existing TCP socket.
    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        self.network.send_internal(&CloseSocket { socket }, false)?;
        self.sockets.try_borrow_mut()?.remove(socket)?;
        Ok(())
    }
}
