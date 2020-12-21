use super::DataService;
use super::{
    socket::{SocketHandle, SocketSetItem, UdpSocket},
    EgressChunkSize, Error,
};
use crate::command::ip_transport_layer::{
    types::SocketProtocol, CloseSocket, CreateSocket, PrepareUDPSendToDataBinary,
    UDPSendToDataBinary,
};
use atat::typenum::Unsigned;
use embedded_nal::{HostSocketAddr, UdpClientStack};
use heapless::{ArrayLength, Bucket, Pos};

impl<'a, C, N, L> UdpClientStack for DataService<'a, C, N, L>
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
    fn socket(&self) -> Result<Self::UdpSocket, Self::Error> {
        let socket_resp = self.network.send_internal(
            &CreateSocket {
                protocol: SocketProtocol::UDP,
                local_port: None,
            },
            false,
        )?;

        let socket = UdpSocket::new(socket_resp.socket.0);

        Ok(self.sockets.try_borrow_mut()?.add(socket)?)
    }

    fn connect(
        &self,
        socket: &mut Self::UdpSocket,
        remote: HostSocketAddr,
    ) -> Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut udp = sockets
            .get::<UdpSocket<_>>(*socket)
            .map_err(Self::Error::from)?;
        udp.bind(remote.as_socket_addr())
            .map_err(Self::Error::from)?;
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let udp = sockets
            .get::<UdpSocket<_>>(*socket)
            .map_err(Self::Error::from)?;

        if !udp.is_open() {
            return Err(Error::SocketClosed.into());
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
                .map_err(Self::Error::from)?;

            let response = self
                .network
                .send_internal(
                    &UDPSendToDataBinary {
                        data: serde_at::ser::Bytes(chunk),
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

        Ok(())
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn receive(
        &self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<(usize, HostSocketAddr), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut udp = sockets
            .get::<UdpSocket<_>>(*socket)
            .map_err(Self::Error::from)?;

        let response = udp
            .recv_slice(buffer)
            .map(|n| (n, udp.endpoint().into()))
            .map_err(Self::Error::from)?;
        Ok(response)
    }

    /// Close an existing UDP socket.
    fn close(&self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        self.network.send_internal(&CloseSocket { socket }, false)?;
        self.sockets.try_borrow_mut()?.remove(socket)?;
        Ok(())
    }
}
