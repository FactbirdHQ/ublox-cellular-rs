use embedded_hal::digital::v2::OutputPin;
pub use embedded_nal::{Ipv4Addr, Mode, SocketAddr, SocketAddrV4};
use embedded_nal::{TcpStack, UdpStack};

use crate::command::ip_transport_layer::{types::*, *};
use crate::error::Error;
use crate::GSMClient;

use crate::socket::{self, AnySocket, Socket, SocketHandle};

#[cfg(feature = "socket-udp")]
use crate::socket::UdpSocket;
#[cfg(feature = "socket-tcp")]
use crate::socket::{TcpSocket, TcpState};

// #[cfg(feature = "socket-udp")]
// impl<C, RST, DTR> UdpStack for GSMClient<C, RST, DTR>
// where
//     C: atat::AtatClient,
//     RST: OutputPin,
//     DTR: OutputPin,
// {
//     type Error = Error;
//     type UdpSocket = UdpSocket;

//     /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
//     fn open(&mut self, _mode: Mode) -> Result<Self::UdpSocket, Self::Error> {
//         let socket_resp = self.send_at(&CreateSocket {
//             protocol: SocketProtocol::UDP,
//             local_port: None,
//         })?;

//         Ok(UdpSocket::new(socket_resp.socket.0))
//     }

//     /// Write to the stream. Returns the number of bytes written is returned
//     /// (which may be less than `buffer.len()`), or an error.
//     fn write(
//         &mut self,
//         socket: &mut Self::UdpSocket,
//         buffer: &[u8],
//     ) -> nb::Result<usize, Self::Error> {
//         if !socket.may_send() {
//             return Err(nb::Error::Other(Error::SocketClosed));
//         }

//         let mut remaining = buffer.len();
//         let mut written = 0;

//         while remaining > 0 {
//             let chunk_size = core::cmp::min(remaining, 256);

//             let mut data = Vec::new();
//             data.extend_from_slice(&buffer[written..written + chunk_size])
//                 .ok();

//             self.send_at(&WriteSocketData {
//                 socket: socket.handle(),
//                 length: chunk_size,
//                 data,
//             })
//             .map_err(|_e| Error::E)?;

//             written += chunk_size;
//             remaining -= chunk_size;
//         }

//         Ok(written)
//     }

//     /// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
//     /// data have been received and they have been placed in
//     /// `&buffer[0..n]`, or an error.
//     fn read(
//         &mut self,
//         socket: &mut Self::UdpSocket,
//         buffer: &mut [u8],
//     ) -> nb::Result<usize, Self::Error> {
//         let data = self.send_at(&ReadSocketData {
//             socket: socket.handle(),
//             length: 256,
//         })?;

//         Ok(data.length)
//     }

//     /// Close an existing UDP socket.
//     fn close(&mut self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
//         // socket.close();

//         self.send_at(&CloseSocket {
//             socket: socket.handle(),
//         })?;

//         Ok(())
//     }
// }

#[cfg(feature = "socket-tcp")]
impl<C, RST, DTR> TcpStack for GSMClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GSMClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type TcpSocket = SocketHandle;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn open(&self, _mode: Mode) -> Result<Self::TcpSocket, Self::Error> {
        let socket_resp = self.send_at(&CreateSocket {
            protocol: SocketProtocol::TCP,
            local_port: None,
        })?;

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
        self.send_at(&ConnectSocket {
            socket,
            remote_addr: remote.ip(),
            remote_port: remote.port(),
        })?;

        let mut sockets = self.sockets.try_borrow_mut()?;
        let mut tcp = sockets.get::<TcpSocket>(socket)?;
        tcp.set_state(TcpState::Established);
        Ok(tcp.handle())
    }

    /// Check if this socket is still connected
    fn is_connected(&self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        let mut sockets = self.sockets.try_borrow_mut()?;
        let tcp = sockets.get::<TcpSocket>(socket.clone())?;
        Ok(tcp.is_active())
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn write(&self, socket: &mut Self::TcpSocket, buffer: &[u8]) -> nb::Result<usize, Self::Error> {
        let mut sockets = self
            .sockets
            .try_borrow_mut()
            .map_err(|e| nb::Error::Other(e.into()))?;

        let tcp = sockets
            .get::<TcpSocket>(socket.clone())
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        if !tcp.is_active() || !tcp.may_send() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        let mut remaining = buffer.len();
        let mut written = 0;

        while remaining > 0 {
            let chunk_size = core::cmp::min(remaining, 256);

            self.send_at(&WriteSocketData {
                socket,
                length: chunk_size,
                data: unsafe {
                    core::str::from_utf8_unchecked(&buffer[written..written + chunk_size])
                },
            })?;

            written += chunk_size;
            remaining -= chunk_size;
        }

        return Ok(written);
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
            .get::<TcpSocket>(socket.clone())
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;
        return tcp
            .recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()));
    }

    /// Close an existing TCP socket.
    fn close(&self, mut socket: Self::TcpSocket) -> Result<(), Self::Error> {
        // socket.close();

        self.send_at(&CloseSocket { socket })?;

        Ok(())
    }
}
