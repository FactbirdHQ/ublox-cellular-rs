//! 4 Responses for Packet Switched Data Services Commands
use super::types::*;
use atat::atat_derive::AtatResp;

// 18.7 Packet switched data configuration +UPSD Sets or reads all the
//  parameters in a specific packet switched data (PSD) profile. The command is
//  used to set up the PDP context parameters for an internal context, i.e. a
//  data connection using the internal IP stack and related AT commands for
//  sockets. To set all the parameters of the PSD profile a set command for each
//  parameter needs to be issued.
#[derive(AtatResp)]
pub struct PacketSwitchedConfig {
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub param: PacketSwitchedParam,
}

/// 18.9 Packet switched network-assigned data +UPSND Returns the current
/// (dynamic) network-assigned or network-negotiated value of the specified
/// parameter for the active PDP context associated with the specified PSD
/// profile.
#[derive(Debug, AtatResp)]
pub struct PacketSwitchedNetworkData {
    #[at_arg(position = 0)]
    pub profile: u8,
    #[at_arg(position = 1)]
    pub param: PacketSwitchedNetworkDataParam,
    #[at_arg(position = 2)]
    pub param_tag: u8, // TODO: Create struct to contain
}

/// 18.14 GPRS attach or detach +CGATT Register (attach) the MT to, or
/// deregister (detach) the MT from the GPRS service. After this command the MT
/// remains in AT command mode. If the MT is already in the requested state
/// (attached or detached), the command is ignored and OK result code is
/// returned. If the requested state cannot be reached, an error result code is
/// returned. The command can be aborted if a character is sent to the DCE
/// during the command execution. Any active PDP context will be automatically
/// deactivated when the GPRS registration state changes to detached.
#[derive(AtatResp)]
pub struct GPRSAttached {
    #[at_arg(position = 0)]
    pub state: GPRSAttachedState,
}
