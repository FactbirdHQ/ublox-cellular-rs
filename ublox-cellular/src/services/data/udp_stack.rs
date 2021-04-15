use core::convert::TryInto;

use super::DataService;
use super::{
    socket::{Error as SocketError, Socket, SocketHandle, UdpSocket},
    EgressChunkSize, Error,
};
use crate::command::ip_transport_layer::{
    types::SocketProtocol, CloseSocket, CreateSocket, PrepareUDPSendToDataBinary,
    UDPSendToDataBinary,
};
use atat::typenum::Unsigned;
use embedded_nal::{SocketAddr, UdpClient};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};
use heapless::{ArrayLength, Bucket, Pos};

impl<'a, C, CLK, N, L> UdpClient for DataService<'a, C, CLK, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
    N: 'static
        + ArrayLength<Option<Socket<L, CLK>>>
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
        let mut sockets = self.sockets.try_borrow_mut()?;

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
                protocol: SocketProtocol::UDP,
                local_port: None,
            },
            false,
        )?;

        Ok(sockets.add(UdpSocket::new(socket_resp.socket.0))?)
    }

    fn connect(&self, socket: &mut Self::UdpSocket, remote: SocketAddr) -> Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut udp = sockets
            .get::<UdpSocket<_, _>>(*socket)
            .map_err(Self::Error::from)?;
        udp.bind(remote).map_err(Self::Error::from)?;
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let udp = sockets
            .get::<UdpSocket<_, _>>(*socket)
            .map_err(Self::Error::from)?;

        if !udp.is_open() {
            return Err(Error::SocketClosed.into());
        }

        for chunk in buffer.chunks(EgressChunkSize::to_usize()) {
            defmt::trace!("Sending: {} bytes", chunk.len());
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
    ) -> nb::Result<(usize, SocketAddr), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut udp = sockets
            .get::<UdpSocket<_, _>>(*socket)
            .map_err(Self::Error::from)?;

        let response = udp
            .recv_slice(buffer)
            .map(|n| (n, udp.endpoint()))
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
