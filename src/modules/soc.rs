use embedded_hal::digital::v2::OutputPin;
pub use embedded_nal::{Ipv4Addr, Mode, SocketAddr, SocketAddrV4};

use crate::command::ip_transport_layer::{types::*, *};
use crate::error::Error;
use crate::modules::ssl::SSL;
use crate::GSMClient;

use crate::socket::SocketHandle;

#[cfg(feature = "socket-udp")]
use crate::socket::UdpSocket;
use embedded_nal::UdpStack;
#[cfg(feature = "socket-tcp")]
use crate::socket::{TcpSocket, TcpState};
use embedded_nal::TcpStack;

#[cfg(feature = "socket-udp")]
impl<C, RST, DTR> UdpStack for GSMClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = Error;

    // Only return a SocketHandle to reference into the SocketSet owned by the GSMClient,
    // as the Socket object itself provides no value without accessing it though the client.
    type UdpSocket = SocketHandle;

    /// Open a new UDP socket to the given address and port. UDP is connectionless,
    /// so unlike `TcpStack` no `connect()` is required.
    fn open(&self, remote: SocketAddr, _mode: Mode) -> Result<Self::UdpSocket, Self::Error> {
        let socket_resp = self.send_at(&CreateSocket {
            protocol: SocketProtocol::UDP,
            local_port: None,
        })?;

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
            .get::<UdpSocket>(socket.clone())
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;

        if !udp.is_open() {
            return Err(nb::Error::Other(Error::SocketClosed));
        }

        let mut remaining = buffer.len();
        let mut written = 0;

        while remaining > 0 {
            let chunk_size = core::cmp::min(remaining, 200);

            self.send_at(&PrepareUDPSendToDataBinary {
                socket: socket.clone(),
                remote_addr: udp.endpoint.ip(),
                remote_port: udp.endpoint.port(),
                length: chunk_size,
            })?;

            let response = self.send_at(&UDPSendToDataBinary {
                data: serde_at::ser::Bytes(&buffer[written..written + chunk_size]),
            })?;

            if response.length != chunk_size {
                return Err(nb::Error::Other(Error::BadLength));
            }
            if &response.socket != socket {
                return Err(nb::Error::Other(Error::WrongSocketType));
            }

            written += chunk_size;
            remaining -= chunk_size;
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
            .get::<UdpSocket>(socket.clone())
            .map_err(|e| nb::Error::Other(Error::Socket(e)))?;
        return udp
            .recv_slice(buffer)
            .map_err(|e| nb::Error::Other(e.into()));
    }

    /// Close an existing UDP socket.
    fn close(&self, socket: Self::UdpSocket) -> Result<(), Self::Error> {
        self.send_at(&CloseSocket { socket })?;

        let mut sockets = self.sockets.try_borrow_mut()?;
        let mut udp = sockets.get::<UdpSocket>(socket)?;
        udp.close()?;

        Ok(())
    }
}

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
        self.enable_ssl(socket, 0)?;

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
        {
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
        }

        let mut remaining = buffer.len();
        let mut written = 0;

        while remaining > 0 {
            let chunk_size = core::cmp::min(remaining, 200);

            self.send_at(&PrepareWriteSocketDataBinary {
                socket: socket.clone(),
                length: chunk_size,
            })?;

            let response = self.send_at(&WriteSocketDataBinary {
                data: serde_at::ser::Bytes(&buffer[written..written + chunk_size]),
            })?;

            if response.length != chunk_size {
                return Err(nb::Error::Other(Error::BadLength));
            }
            if &response.socket != socket {
                return Err(nb::Error::Other(Error::WrongSocketType));
            }

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
        self.spin()?;

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
    fn close(&self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        // socket.close();

        self.send_at(&CloseSocket { socket })?;

        Ok(())
    }
}
