pub use embedded_nal::{TcpStack, Mode, IpAddress, Port};
use crate::GSMClient;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::timer::CountDown;

use crate::command::*;

#[derive(Debug)]
pub enum Error {
    E
}

pub enum SocketState {
    Unconnected,
    Connected
}

pub struct TcpSocket {
    state: SocketState,
    mode: Mode,
    socket_id: u8
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
        self.send_at(Command::CreateSocket {
            protocol: SocketProtocol::TCP
        });

        Ok(TcpSocket {
            state: SocketState::Unconnected,
            mode,
            socket_id: 0,
        })
    }

	/// Connect to the given remote host and port.
	fn connect(host: IpAddress, port: Port) -> Result<Self::TcpSocket, Self::Error> {
        Err(Error::E)
    }

	/// Check if this socket is still connected
	fn is_connected(&self, socket: &Self::TcpSocket) -> Result<bool, Self::Error> {
        Err(Error::E)

    }

	/// Write to the stream. Returns the number of bytes written is returned
	/// (which may be less than `buffer.len()`), or an error.
	fn write(&self, socket: &mut Self::TcpSocket, buffer: &[u8]) -> nb::Result<usize, Self::Error> {
        Err(nb::Error::Other(Error::E))
    }

	/// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
	/// data have been received and they have been placed in
	/// `&buffer[0..n]`, or an error.
	fn read(&self, socket: &mut Self::TcpSocket, buffer: &mut [u8]) -> nb::Result<usize, Self::Error> {
        Err(nb::Error::Other(Error::E))
    }

	/// Close an existing TCP socket.
	fn close(socket: Self::TcpSocket) -> Result<(), Self::Error> {
        Err(Error::E)

    }

}
