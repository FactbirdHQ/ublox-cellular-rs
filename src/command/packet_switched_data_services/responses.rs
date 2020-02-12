//! 4 Responses for General Commands
use heapless::{consts, String};
use serde::Deserialize;
pub struct NoResponse;



// 18.7 Packet switched data configuration +UPSD
/// Sets or reads all the parameters in a specific packet switched data (PSD) profile. The command is used to set
/// up the PDP context parameters for an internal context, i.e. a data connection using the internal IP stack and
/// related AT commands for sockets.
/// To set all the parameters of the PSD profile a set command for each parameter needs to be issued.
// #[derive(Deserialize)]
pub struct PacketSwitchedConfig {
    // #[atat_(position = 0)]
    profile_id: u8,
    // #[atat_(position = 1)]
    param: PacketSwitchedParam,
}

/// 18.9 Packet switched network-assigned data +UPSND
/// Returns the current (dynamic) network-assigned or network-negotiated value of the specified parameter for
/// the active PDP context associated with the specified PSD profile.
// #[derive(Deserialize)]
pub struct PacketSwitchedNetworkData{
    // #[atat_(position = 0)]
    profile: u8,
    // #[atat_(position = 1)]
    param: PacketSwitchedNetworkDataParam,
    // #[atat_(position = 2)]
    param_tag : String<consts::U64>,        //TODO: Create struct to contain 
}

/// 18.14 GPRS attach or detach +CGATT
/// Register (attach) the MT to, or deregister (detach) the MT from the GPRS service. After this command the MT
/// remains in AT command mode. If the MT is already in the requested state (attached or detached), the command
/// is ignored and OK result code is returned. If the requested state cannot be reached, an error result code is
/// returned. The command can be aborted if a character is sent to the DCE during the command execution. Any
/// active PDP context will be automatically deactivated when the GPRS registration state changes to detached.
// #[derive(Deserialize)]
pub struct GPRSAttached {
    // #[atat_(position = 1)]
    state : GPRSAttachedState
}