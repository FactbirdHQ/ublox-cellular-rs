use super::DataService;
use super::{
    socket::{SocketHandle, SocketSetItem, UdpSocket},
    EgressChunkSize, Error,
};
use crate::command::ip_transport_layer::{
    types::SocketProtocol, CloseSocket, CreateSocket, PrepareUDPSendToDataBinary,
    UDPSendToDataBinary,
};
use crate::network::Error as NetworkError;
use atat::typenum::Unsigned;
use embedded_nal::{Mode, SocketAddr, UdpStack};
use heapless::{ArrayLength, Bucket, Pos};

impl<'a, C, N, L> UdpStack for DataService<'a, C, N, L>
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
    type UdpSocket = SocketHandle;

    /// Open a new UDP socket to the given address and port. UDP is connectionless,
    /// so unlike `TcpStack` no `connect()` is required.
    fn open(&self, remote: SocketAddr, _mode: Mode) -> Result<Self::UdpSocket, Self::Error> {
        if !self.network.is_registered()?.is_some() {
            return Err(Error::Network(NetworkError::_Unknown));
        }

        let socket_resp = self.network.send_internal(
            &CreateSocket {
                protocol: SocketProtocol::UDP,
                local_port: None,
            },
            false,
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
            .get::<UdpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        if !udp.is_open() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            defmt::trace!("Sending: {:?} bytes, {:?}", chunk.len(), chunk);
            self.network
                .send_internal(
                    &PrepareUDPSendToDataBinary {
                        socket: *socket,
                        remote_addr: udp.endpoint.ip(),
                        remote_port: udp.endpoint.port(),
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
                    &UDPSendToDataBinary {
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

        Ok(())
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn read(
        &self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let mut udp = sockets
            .get::<UdpSocket<_>>(*socket)
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        udp.recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()))
    }

    /// Close an existing UDP socket.
    fn close(&self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        self.network.send_internal(&CloseSocket { socket }, false)?;
        self.sockets.try_borrow_mut()?.remove(socket)?;
        Ok(())
    }
}
