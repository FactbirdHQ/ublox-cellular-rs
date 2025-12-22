//! Unsolicited responses for Internet protocol transport layer Commands
use atat::atat_derive::AtatResp;

#[cfg(feature = "internal-network-stack")]
use ublox_sockets::SocketHandle;

#[cfg(feature = "internal-network-stack")]
/// +UUSORD/+UUSORF
#[derive(Debug, Clone, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketDataAvailable {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}

/// +UUSOCL (always available as it can be sent even in PPP mode)
#[derive(Debug, Clone, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketClosed {
    #[at_arg(position = 0)]
    pub socket: u8,  // Use u8 directly instead of SocketHandle to avoid dependency on ublox-sockets
}
