//! Unsolicited responses for Internet protocol transport layer Commands
use atat::atat_derive::AtatResp;

#[cfg(feature = "internal-network-stack")]
use ublox_sockets::SocketHandle;

/// +UUSORD/+UUSORF
#[cfg(feature = "internal-network-stack")]
#[derive(Debug, Clone, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketDataAvailable {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
    #[at_arg(position = 1)]
    pub length: usize,
}

/// +UUSOCL
#[cfg(feature = "internal-network-stack")]
#[derive(Debug, Clone, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketClosed {
    #[at_arg(position = 0)]
    pub socket: SocketHandle,
}

/// +UUSOCL (for PPP mode without internal-network-stack)
#[cfg(not(feature = "internal-network-stack"))]
#[derive(Debug, Clone, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SocketClosed {
    #[at_arg(position = 0)]
    pub socket: u8,
}
