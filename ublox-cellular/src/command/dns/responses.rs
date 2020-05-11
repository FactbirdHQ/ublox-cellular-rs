//! Responses for DNS Commands
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 24.1 Resolve name / IP number through DNS +UDNSRN
#[derive(Clone, PartialEq, AtatResp)]
pub struct ResolveNameIpResponse {
    #[at_arg(position = 0)]
    pub ip_domain_string: String<consts::U128>,
}
