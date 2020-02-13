//! AT Commands for U-Blox short range module family\
//! Following [ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

pub mod general;
pub mod gpio;
pub mod ip_transport_layer;
pub mod mobile_control;
pub mod packet_switched_data_services;
pub mod system_features;

use atat::{atat_derive::ATATResp, ATATResp};

#[derive(Clone, ATATResp)]
pub struct NoResponse;
