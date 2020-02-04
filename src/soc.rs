use crate::GSMClient;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::timer::CountDown;
pub use embedded_nal::{IpAddress, Mode, Port, TcpStack};

use crate::command::*;

#[derive(Debug)]
pub enum Error {
    E,
    AT(at::Error),
}

impl From<at::Error> for Error {
    fn from(e: at::Error) -> Self {
        Error::AT(e)
    }
}

pub enum SocketState {
    Unconnected,
    Connected,
}

pub struct TcpSocket {
    state: SocketState,
    mode: Mode,
    socket_id: u8,
}

impl<T, U, RST, DTR> TcpStack for GSMClient<T, RST, DTR>
where
    T: CountDown<Time = U>,
    U: From<u32>,
    T::Time: Copy,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = Error;
    type TcpSocket = TcpSocket;

    /// Open a new TCP socket to the given address and port. The socket starts in the unconnected state.
    fn open(&mut self, mode: Mode) -> Result<Self::TcpSocket, Self::Error> {
        let socket_resp = self.send_at(Command::CreateSocket {
            protocol: SocketProtocol::TCP,
        })?;

        if let ResponseType::SingleSolicited(r) = socket_resp {
            Ok(TcpSocket {
                state: SocketState::Unconnected,
                mode,
                socket_id: 0,
            })
        } else {
            Err(Error::E)
        }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: &Self::TcpSocket,
        host: IpAddress,
        port: Port,
    ) -> Result<Self::TcpSocket, Self::Error> {
        self.send_at(Command::ConnectSocket {
            socket: socket.socket_id,
            remote_addr: host,
            remote_port: port,
        })?;

        Ok(TcpSocket {
            state: SocketState::Connected,
            ..*socket
        })
    }

    /// Check if this socket is still connected
    fn is_connected(&mut self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        Err(Error::E)
    }

    /// Write to the stream. Returns the number of bytes written is returned
    /// (which may be less than `buffer.len()`), or an error.
    fn write(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &[u8],
    ) -> nb::Result<usize, Self::Error> {
        Err(nb::Error::Other(Error::E))
    }

    /// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
    /// data have been received and they have been placed in
    /// `&buffer[0..n]`, or an error.
    fn read(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        Err(nb::Error::Other(Error::E))
    }

    /// Close an existing TCP socket.
    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        self.send_at(Command::CloseSocket {
            socket: socket.socket_id,
        })?;

        Ok(())
    }
}
