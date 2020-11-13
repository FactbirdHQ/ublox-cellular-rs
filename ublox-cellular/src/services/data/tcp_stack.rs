use super::ssl::{SecurityProfileId, SSL};
use super::DataService;
use super::{
    socket::{Error as SocketError, SocketHandle, SocketSetItem, TcpSocket, TcpState},
    EgressChunkSize, Error,
};
use crate::command::ip_transport_layer::{
    types::SocketProtocol, CloseSocket, ConnectSocket, CreateSocket, PrepareWriteSocketDataBinary,
    WriteSocketDataBinary,
};
use crate::network::Error as NetworkError;
use atat::typenum::Unsigned;
use embedded_nal::{Mode, SocketAddr, TcpStack};
use heapless::{ArrayLength, Bucket, Pos};

impl<'a, C, N, L> TcpStack for DataService<'a, C, N, L>
where
    C: atat::AtatClient,
    N: 'static
        + ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn open(&self, _mode: Mode) -> Result<Self::TcpSocket, Self::Error> {
        if self.network.is_registered()?.is_none() {
            return Err(Error::Network(NetworkError::_Unknown));
        }

        let socket_resp = self.network.send_internal(
            &CreateSocket {
                protocol: SocketProtocol::TCP,
                local_port: None,
            },
            true,
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
        let mut sockets = self.sockets.try_borrow_mut()?;
        let mut tcp = sockets.get::<TcpSocket<_>>(socket)?;

        if tcp.state() == TcpState::Created {
            self.enable_ssl(socket, SecurityProfileId(0))?;

            self.network.send_internal(
                &ConnectSocket {
                    socket,
                    remote_addr: remote.ip(),
                    remote_port: remote.port(),
                },
                false,
            )?;

            tcp.set_state(TcpState::Connected);
            Ok(tcp.handle())
        } else {
            defmt::error!("Cannot connect socket! Socket state: {:?}", tcp.state());
            Err(Error::Socket(SocketError::Illegal))
        }
    }

    /// Check if this socket is still connected
    fn is_connected(&self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut()?;
        Ok(sockets.get::<TcpSocket<_>>(*socket)?.is_active())
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn write(&self, socket: &mut Self::TcpSocket, buffer: &[u8]) -> nb::Result<usize, Self::Error> {
        if !self.is_connected(&socket)? {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            defmt::trace!("Sending: {:?} bytes, {:?}", chunk.len(), chunk);
            self.network
                .send_internal(
                    &PrepareWriteSocketDataBinary {
                        socket: *socket,
                        length: chunk.len(),
                    },
                    false,
                )
                .map_err(|e| nb::Error::Other(e.into()))?;

            // self.delay
            //     .try_borrow_mut()
            //     .map_err(|_| Error::BorrowMutError)?
            //     .try_delay_ms(50)
            //     .map_err(|_| Error::Busy)?;

            let response = self
                .network
                .send_internal(
                    &WriteSocketDataBinary {
                        data: serde_at::ser::Bytes(chunk),
                    },
                    false,
                )
                .map_err(|e| nb::Error::Other(e.into()))?;

            if response.length != chunk.len() {
                return Err(nb::Error::Other(Error::BadLength));
            }
            if &response.socket != socket {
                return Err(nb::Error::Other(Error::WrongSocketType));
            }
        }

        Ok(buffer.len())
    }

    /// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
    /// data have been received and they have been placed in
    /// `&buffer[0..n]`, or an error.
    fn read(
        &self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;
        let mut tcp = sockets
            .get::<TcpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(e.into()))?;

        tcp.recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))
    }

    fn read_with<F>(&self, socket: &mut Self::TcpSocket, f: F) -> nb::Result<usize, Self::Error>
    where
        F: FnOnce(&[u8], Option<&[u8]>) -> usize,
    {
        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;
        let mut tcp = sockets
            .get::<TcpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(e.into()))?;

        tcp.recv_wrapping(|a, b| f(a, b))
            .map_err(|e| nb::Error::Other(e.into()))
    }

    /// Close an existing TCP socket.
    fn close(&self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        self.network.send_internal(&CloseSocket { socket }, false)?;
        self.sockets.try_borrow_mut()?.remove(socket)?;
        Ok(())
    }
}
