//! 4 General Commands
pub mod responses;
pub mod types;
use atat::{Error, atat_derive::ATATCmd, ATATCmd};
use heapless::{consts, String, Vec};
use responses::*;
use types::*;

use super::NoResponse;
use crate::socket::SocketHandle;


/// 25.3 Create Socket +USOCR
/// Creates a socket and associates it with the specified protocol (TCP or UDP), returns a number identifying the
/// socket. Such command corresponds to the BSD socket routine:
/// • TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 / SARA-G4 / SARA-G340 /
/// SARA-G350 - Up to 7 sockets can be created.
/// • LEON-G1 - Up to 16 sockets can be created
/// It is possible to specify the local port to bind within the socket in order to send data from a specific port. The
/// bind functionality is supported for both TCP and UDP sockets.
#[derive(Clone, ATATCmd)]
#[at_cmd("+USOCR", CreateSocketResponse)]
pub struct CreateSocket {
    #[at_arg(position = 0)]
    pub protocol: SocketProtocol,
    #[at_arg(position = 1)]
    pub local_port : Option<u16>,
}


/// 25.7 Close Socket +USOCL
/// Closes the specified socket, like the BSD close routine. In case of remote socket closure the user is notified
/// via the URC.
/// By default the command blocks the AT command interface until the the completion of the socket close
/// operation. By enabling the <async_close> flag, the final result code is sent immediately. The following
/// +UUSOCL URC will indicate the closure of the specified socket.
#[derive(Clone, ATATCmd)]
#[at_cmd("+USOCL", NoResponse)]
pub struct CloseSocket {
    #[at_arg(position = 0)]
    pub socket: SocketHandle
}

/// 25.8 Get Socket Error +USOER
/// Retrieves the last error occurred in the last socket operation, stored in the BSD standard variable error.
#[derive(Clone, ATATCmd)]
#[at_cmd("+USOER", SocketErrorResponse)]
pub struct GetSocketError;




/// 25.9 Connect Socket +USOCO
/// Establishes a peer-to-peer connection of the socket to the specified remote host on the given remote port, like
/// the BSD connect routine. If the socket is a TCP socket, the command will actually perform the TCP negotiation
/// (3-way handshake) to open a connection. If the socket is a UDP socket, this function will just declare the remote
/// host address and port for later use with other socket operations (e.g. +USOWR, +USORD). This is important
/// to note because if <socket> refers to a UDP socket, errors will not be reported prior to an attempt to write or
/// read data on the socket.
#[derive(Clone, ATATCmd)]
#[at_cmd("+USOCO", NoResponse)]
pub struct ConnectSocket {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    // #[at_arg(position = 1)]
    // pub remote_addr: IpAddress, //Todo: Import struct from new lib
    #[at_arg(position = 2)]
    pub remote_port: u16,
}


/// 25.10 Write socket data +USOWR
/// Writes the specified amount of data to the specified socket, like the BSD write routine, and returns the number
/// of bytes of data actually written. The command applies to UDP sockets too, after a +USOCO command.
/// There are three kinds of syntax:
/// • Base syntax normal: writing simple strings to the socket, some characters are forbidden
/// • Base syntax HEX: writing hexadecimal strings to the socket, the string will be converted in binary data and
/// sent to the socket; see the AT+UDCONF=1 command description to enable it
/// • Binary extended syntax: mandatory for writing any character in the ASCII range [0x00, 0xFF]
#[derive(Clone, ATATCmd)]
#[at_cmd("+USOWR", WriteSocketDataResponse, timeout_ms = 1000)]
pub struct WriteSocketData {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
    #[at_arg(position = 2)]
    pub data: Vec<u8, consts::U256>
}


/// 25.12 Read Socket Data +USORD
/// Reads the specified amount of data from the specified socket, like the BSD read routine. This command can
/// be used to know the total amount of unread data.
/// For the TCP socket type the URC +UUSORD: <socket>,<length> notifies the data bytes available for reading,
///  either when buffer is empty and new data arrives or after a partial read by the user.
/// For the UDP socket type the URC +UUSORD: <socket>,<length> notifies that a UDP packet has been received,
///  either when buffer is empty or after a UDP packet has been read and one or more packets are stored in the
/// buffer.
/// In case of a partial read of a UDP packet +UUSORD: <socket>,<length> will show the remaining number of data
/// bytes of the packet the user is reading.
#[derive(Clone, ATATCmd)]
#[at_cmd("+USORD", SocketData, timeout_ms = 10000, abortable = true)]
pub struct ReadSocketData {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}
