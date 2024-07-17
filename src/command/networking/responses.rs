//! Responses for Networking Commands
use atat::atat_derive::AtatResp;

/// 34.4 Configure port filtering for embedded applications +UEMBPF
#[derive(Debug, Clone, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct EmbeddedPortFiltering {
    #[at_arg(position = 0)]
    pub mode: u8,
    #[at_arg(position = 1)]
    pub port_range: heapless::String<16>,
}
