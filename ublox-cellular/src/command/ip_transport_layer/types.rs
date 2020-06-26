//! Argument and parameter types used by Internet protocol transport layer Commands and Responses
use atat::atat_derive::AtatEnum;

#[derive(Clone, PartialEq, AtatEnum)]
pub enum SocketProtocol {
    TCP = 6,
    UDP = 17,
}

#[derive(Clone, PartialEq, AtatEnum)]
// TODO: Enabled(u8), once serde_at handles tuples, see https://github.com/BlackbirdHQ/atat/issues/37
pub enum SslTlsStatus {
    /// 0 (default value): disable the SSL/TLS on the socket
    Disabled = 0,
    /// 1: enable the SSL/TLS on the socket; a USECMNG profile can be specified
    /// with the <usecmng_profile_id> parameter.
    Enabled = 1,
}

/// Enables/disables the HEX mode for +USOWR, +USOST, +USORD and +USORF AT
/// commands.
#[derive(Clone, PartialEq, AtatEnum)]
pub enum HexMode {
    /// 0 (factory-programmed value): HEX mode disabled
    Disabled = 0,
    /// 1: HEX mode enabled
    Enabled = 1,
}

/// Control request identifier
#[derive(Clone, PartialEq, AtatEnum)]
pub enum SocketControlParam {
    /// 0: query for socket type
    SocketType = 0,
    /// 1: query for last socket error
    LastSocketError = 1,
    /// 2: get the total amount of bytes sent from the socket
    BytesSent = 2,
    /// 3: get the total amount of bytes received by the socket
    BytesReceived = 3,
    /// 4: query for remote peer IP address and port
    RemotePeerSocketAddr = 4,
    /// 10: query for TCP socket status (only TCP sockets)
    SocketStatus = 10,
    /// 11: query for TCP outgoing unacknowledged data (only TCP sockets)
    OutgoingUnackData = 11,
    // /// 5-9, 12-99: RFU
}
