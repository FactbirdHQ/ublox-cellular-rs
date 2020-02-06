use embedded_hal::{digital::v2::OutputPin, timer::CountDown};
use embedded_nal::TcpStack;
pub use embedded_nal::{IpAddress, Ipv4Address, Mode, Port};

use crate::command::*;
use crate::GSMClient;

use crate::socket::{SocketHandle, TcpSocket, TcpState};
use heapless::Vec;

use log::info;

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

        // if let ResponseType::SingleSolicited(r) = socket_resp {
        Ok(TcpSocket::new(0))
        // } else {
        //     Err(Error::E)
        // }
    }

    /// Connect to the given remote host and port.
    fn connect(
        &mut self,
        socket: Self::TcpSocket,
        host: IpAddress,
        port: Port,
    ) -> Result<Self::TcpSocket, Self::Error> {
        self.send_at(Command::ConnectSocket {
            socket: socket.handle(),
            remote_addr: host,
            remote_port: port,
        })?;

        let mut ret_soc = socket;
        ret_soc.set_state(TcpState::Established);
        Ok(ret_soc)
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
        let mut remaining = buffer.len();
        let mut written = 0;

        while remaining > 0 {
            let chunk_size = core::cmp::min(remaining, 256);

            let mut data = Vec::new();
            data.extend_from_slice(&buffer[written..written + chunk_size])
                .ok();

            self.send_at(Command::WriteSocketData {
                socket: socket.handle(),
                length: chunk_size,
                data,
            })
            .map_err(|_e| Error::E)?;

            written += chunk_size;
            remaining -= chunk_size;
        }

        Ok(written)
    }

    /// Read from the stream. Returns `Ok(n)`, which means `n` bytes of
    /// data have been received and they have been placed in
    /// `&buffer[0..n]`, or an error.
    fn read(
        &mut self,
        socket: &mut Self::TcpSocket,
        buffer: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        self.send_at(Command::ReadSocketData {
            socket: socket.handle(),
            length: 256,
        })
        .map_err(|_e| Error::E)?;

        Ok(256)
    }

    /// Close an existing TCP socket.
    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        self.send_at(Command::CloseSocket {
            socket: socket.handle(),
        })?;

        Ok(())
    }
}
