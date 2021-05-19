use core::convert::TryInto;

use super::DataService;
use super::{
    socket::{Error as SocketError, SocketHandle, UdpSocket},
    EGRESS_CHUNK_SIZE, Error,
};
use crate::command::ip_transport_layer::{
    types::SocketProtocol, CloseSocket, CreateSocket, PrepareUDPSendToDataBinary,
    UDPSendToDataBinary,
};
use embedded_nal::{SocketAddr, UdpClientStack};
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock,
};

impl<'a, C, CLK, const N: usize, const L: usize> UdpClientStack for DataService<'a, C, CLK, N, L>
where
    C: atat::AtatClient,
    CLK: Clock,
    Generic<CLK::T>: TryInto<Milliseconds>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    /// Open a new UDP socket to the given address and port. UDP is connectionless,
    /// so unlike `TcpStack` no `connect()` is required.
    fn socket(&mut self) -> Result<Self::UdpSocket, Self::Error> {
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

    fn connect(&mut self, socket: &mut Self::UdpSocket, remote: SocketAddr) -> Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut udp = sockets
            .get::<UdpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?;
        udp.bind(remote).map_err(Self::Error::from)?;
        Ok(())
    }

    /// Send a datagram to the remote host.
    fn send(&mut self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let udp = sockets
            .get::<UdpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?;

        if !udp.is_open() {
            return Err(Error::SocketClosed.into());
        }

        for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
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

        Ok(())
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn receive(
        &mut self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<(usize, SocketAddr), Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut().map_err(Self::Error::from)?;

        let mut udp = sockets
            .get::<UdpSocket<CLK, L>>(*socket)
            .map_err(Self::Error::from)?;

        let response = udp
            .recv_slice(buffer)
            .map(|n| (n, udp.endpoint()))
            .map_err(Self::Error::from)?;
        Ok(response)
    }

    /// Close an existing UDP socket.
    fn close(&mut self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        self.network.send_internal(&CloseSocket { socket }, false)?;
        self.sockets.try_borrow_mut()?.remove(socket)?;
        Ok(())
    }
}
