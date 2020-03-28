//! Responses for DNS Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 24.1 Resolve name / IP number through DNS +UDNSRN
#[derive(Clone, Debug, PartialEq, AtatResp)]
pub struct ResolveIpResponse {
    #[at_arg(position = 0)]
    pub ip_string: String<consts::U64>,
}
