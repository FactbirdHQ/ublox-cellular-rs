//! Responses for Device lock Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use atat::AtatResp;
use heapless::{consts, String, Vec};

/// 9.1 Enter PIN +CPIN
#[derive(Clone, Debug, PartialEq, AtatResp)]
pub struct PinStatus {
    #[at_arg(position = 0)]
    pub code: PinStatusCode,
}
