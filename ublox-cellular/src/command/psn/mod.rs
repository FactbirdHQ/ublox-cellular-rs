//! ### 18 - Packet Switched Data Services Commands
//!
//! A PDP context can be either primary or secondary. In LTE, PS data
//! connections are referred to as EPS bearers: EPS bearers are conceptually
//! equivalent to the legacy PDP contexts, which are often referred to for sake
//! of simplicity. Similarly to a PDP context, the EPS bearer can be a default
//! (primary) or dedicated (secondary) one. The initial EPS bearer established
//! during LTE attach procedure is actually a default EPS bearer. A secondary
//! PDP context uses the same IP address of a primary PDP context (the usual PDP
//! context activated e.g. via dial-up). The Traffic Flow Filters for such
//! secondary contexts shall be specified according to 3GPP TS 23.060
//!
//! The typical usage of the secondary PDP contexts is in VoIP calls, where RTP
//! (speech) packets are conveyed on one PDP context (e.g. the primary one) with
//! a given QoS (e.g. low reliability) whereas SIP signalling is routed on a
//! different PDP context (e.g. the secondary one, with the same IP address but
//! different port numbers) with a more reliable QoS.
//!
//! A Traffic Flow Template (i.e. a filter based on port number, specifying
//! relative flow precedence) shall be configured for the secondary context to
//! instruct the GGSN to route down-link packets onto different QoS flows
//! towards the TE.

pub mod responses;
pub mod types;
pub mod urc;
use atat::atat_derive::AtatCmd;
use responses::*;
use types::*;

use super::NoResponse;

/// 18.7 Set Packet switched data configuration +UPSD
///
/// Sets all the parameters in a specific packet switched data (PSD) profile.
/// The command is used to set up the PDP context parameters for an internal
/// context, i.e. a data connection using the internal IP stack and related AT
/// commands for sockets. To set all the parameters of the PSD profile a set
/// command for each parameter needs to be issued.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UPSD", NoResponse)]
pub struct SetPacketSwitchedConfig {
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub param: PacketSwitchedParam,
}

/// 18.7 Get Packet switched data configuration +UPSD
///
/// Gets all the parameters in a specific packet switched data (PSD) profile.
/// The command is used to set up the PDP context parameters for an internal
/// context, i.e. a data connection using the internal IP stack and related AT
/// commands for sockets. To set all the parameters of the PSD profile a set
/// command for each parameter needs to be issued.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UPSD", PacketSwitchedConfig)]
pub struct GetPacketSwitchedConfig {
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub param: PacketSwitchedParamReq, // NOTE: Currently reading all at once is unsupported!
}

/// 18.8 Set Packet switched data action +UPSDA
///
/// Performs the requested action for the specified PSD profile. The command can
/// be aborted. When a PDP context activation (<action>=3) or a PDP context
/// deactivation (<action>=4) is aborted, the +UUPSDA URC is provided. The
/// <result> parameter indicates the operation result. Until this operation is
/// not completed, another set command cannot be issued. The +UUPSDD URC is
/// raised when the data connection related to the provided PSD profile is
/// deactivated either explicitly by the network (e.g. due to prolonged idle
/// time) or locally by the module after a failed PS registration procedure
/// (e.g. due to roaming) or a user required detach (e.g. triggered by
/// AT+COPS=2).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UPSDA", NoResponse, timeout_ms = 180000, abortable = true)]
pub struct SetPacketSwitchedAction {
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub action: PacketSwitchedAction,
}

/// 18.9 Get Packet switched network-assigned data +UPSND
///
/// Returns the current (dynamic) network-assigned or network-negotiated value
/// of the specified parameter for the active PDP context associated with the
/// specified PSD profile.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UPSND", PacketSwitchedNetworkData)]
pub struct GetPacketSwitchedNetworkData {
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub param: PacketSwitchedNetworkDataParam,
}

/// 18.14 Set GPRS attach or detach +CGATT
///
/// Register (attach) the MT to, or deregister (detach) the MT from the GPRS
/// service. After this command the MT remains in AT command mode. If the MT is
/// already in the requested state (attached or detached), the command is
/// ignored and OK result code is returned. If the requested state cannot be
/// reached, an error result code is returned. The command can be aborted if a
/// character is sent to the DCE during the command execution. Any active PDP
/// context will be automatically deactivated when the GPRS registration state
/// changes to detached.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGATT", NoResponse, timeout_ms = 180000, abortable = true)]
pub struct SetGPRSAttached {
    #[at_arg(position = 0)]
    pub state: GPRSAttachedState,
}

/// 18.14 Read GPRS attach or detach +CGATT
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGATT?", GPRSAttached, timeout_ms = 180000, abortable = true)]
pub struct GetGPRSAttached;
