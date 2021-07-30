//! Unsolicited responses for HTTP Commands
use atat::atat_derive::AtatResp;
use heapless::String;

/// 7.14 Network registration status +CREG
#[derive(Debug, Clone, AtatResp)]
pub struct HttpResponse {
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub http_command: u8,
    #[at_arg(position = 2)]
    pub http_result: u8,
}
