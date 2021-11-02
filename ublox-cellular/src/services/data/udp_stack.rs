use super::DataService;
use super::{Clock, Error, EGRESS_CHUNK_SIZE};
use crate::command::ip_transport_layer::{
    types::SocketProtocol, CloseSocket, CreateSocket, PrepareUDPSendToDataBinary,
    UDPSendToDataBinary,
};
use embedded_nal::{SocketAddr, UdpClientStack};
use ublox_sockets::{Error as SocketError, SocketHandle, UdpSocket};

impl<'a, C, CLK, const FREQ_HZ: u32, const N: usize, const L: usize> UdpClientStack
    for DataService<'a, C, CLK, FREQ_HZ, N, L>
where
    C: atat::AtatClient,
    CLK: Clock<FREQ_HZ>,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GsmClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    /// Open a new UDP socket to the given address and port. UDP is connectionless,
    /// so unlike `TcpStack` no `connect()` is required.
    fn socket(&mut self) -> Result<Self::UdpSocket, Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            if sockets.len() >= sockets.capacity() {
                let ts = self.network.status.timer.now();
                // Check if there are any sockets closed by remote, and close it
                // if it has exceeded its timeout, in order to recycle it.
                if sockets.recycle(ts) {
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
        } else {
            Err(Error::SocketMemory)
        }
    }

    fn connect(
        &mut self,
        socket: &mut Self::UdpSocket,
        remote: SocketAddr,
    ) -> Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            let mut udp = sockets
                .get::<UdpSocket<FREQ_HZ, L>>(*socket)
                .map_err(Self::Error::from)?;
            udp.bind(remote).map_err(Self::Error::from)?;
            Ok(())
        } else {
            Err(Error::SocketMemory)
        }
    }

    /// Send a datagram to the remote host.
    fn send(&mut self, socket: &mut Self::UdpSocket, buffer: &[u8]) -> nb::Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            let udp = sockets
                .get::<UdpSocket<FREQ_HZ, L>>(*socket)
                .map_err(Self::Error::from)?;

            if !udp.is_open() {
                return Err(Error::SocketClosed.into());
            }

            for chunk in buffer.chunks(EGRESS_CHUNK_SIZE) {
                defmt::trace!("Sending: {} bytes", chunk.len());
                let endpoint = udp.endpoint().ok_or(Error::SocketClosed)?;
                self.network
                    .send_internal(
                        &PrepareUDPSendToDataBinary {
                            socket: *socket,
                            remote_addr: endpoint.ip(),
                            remote_port: endpoint.port(),
                            length: chunk.len(),
                        },
                        false,
                    )
                    .map_err(Self::Error::from)?;

                let response = self
                    .network
                    .send_internal(
                        &UDPSendToDataBinary {
                            data: atat::serde_bytes::Bytes::new(chunk),
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
        } else {
            Err(Error::SocketMemory.into())
        }
    }

    /// Read a datagram the remote host has sent to us. Returns `Ok(n)`, which
    /// means a datagram of size `n` has been received and it has been placed
    /// in `&buffer[0..n]`, or an error.
    fn receive(
        &mut self,
        socket: &mut Self::UdpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<(usize, SocketAddr), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            let mut udp = sockets
                .get::<UdpSocket<FREQ_HZ, L>>(*socket)
                .map_err(Self::Error::from)?;

            let bytes = udp.recv_slice(buffer).map_err(Self::Error::from)?;

            let endpoint = udp.endpoint().ok_or(Error::SocketClosed)?;
            Ok((bytes, endpoint))
        } else {
            Err(Error::SocketMemory.into())
        }
    }

    /// Close an existing UDP socket.
    fn close(&mut self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        if let Some(ref mut sockets) = self.sockets {
            self.network.send_internal(&CloseSocket { socket }, false)?;
            sockets.remove(socket)?;
            Ok(())
        } else {
            Err(Error::SocketMemory)
        }
    }
}
