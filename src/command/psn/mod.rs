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
//! The typical usage of the secondary PDP contexts is in `VoIP` calls, where RTP
//! (speech) packets are conveyed on one PDP context (e.g. the primary one) with
//! a given `QoS` (e.g. low reliability) whereas SIP signalling is routed on a
//! different PDP context (e.g. the secondary one, with the same IP address but
//! different port numbers) with a more reliable `QoS`.
//!
//! A Traffic Flow Template (i.e. a filter based on port number, specifying
//! relative flow precedence) shall be configured for the secondary context to
//! instruct the GGSN to route down-link packets onto different `QoS` flows
//! towards the TE.

pub mod responses;
pub mod types;
pub mod urc;
use atat::atat_derive::AtatCmd;
use responses::{
    EPSNetworkRegistrationStatus, ExtendedPSNetworkRegistrationStatus, GPRSAttached,
    GPRSNetworkRegistrationStatus, PDPContextState, PacketSwitchedConfig,
    PacketSwitchedNetworkData,
};
use types::{
    AuthenticationType, ContextId, EPSNetworkRegistrationUrcConfig,
    ExtendedPSNetworkRegistrationUrcConfig, GPRSAttachedState, GPRSNetworkRegistrationUrcConfig,
    PDPContextStatus, PSEventReportingMode, PacketSwitchedAction, PacketSwitchedNetworkDataParam,
    PacketSwitchedParam, PacketSwitchedParamReq, ProfileId,
};

use super::NoResponse;

/// 18.4 PDP context definition +CGDCONT
///
/// Defines the connection parameters for a PDP context, identified by the local
/// context identification parameter <cid>. If the command is used only with
/// parameter <cid>, the corresponding PDP context becomes undefined.
///
/// Each context is permanently stored so that its definition is persistent over
/// power cycles.
///
/// The command is used to set up the PDP context parameters for an external
/// context, i.e. a data connection using the external IP stack (e.g. Windows
/// dial-up) and PPP link over the serial interface.
///
/// Usage of static i.e. user defined IP address is possible in UTRAN and GERAN
/// but not in EUTRAN; to prevent inconsistent addressing methods across various
/// RATs, static IP addressing is not recommended for LTE modules: 3GPP TS
/// 23.060 [10] Rel.8 and later releases specify that a UE with
/// EUTRAN/UTRAN/GERAN capabilities shall not include a static PDP address in
/// PDP context activation requests.
///
/// The information text response to the read command provides the configuration
/// of all the PDP context / EPS bearers that have already been defined. The
/// test command returns a different row for each <`PDP_type`> value supported by
/// the module.
///"`IPtial` default bearer. Since dial-up supports only IPv4
///   connectivity, the defined IPv6 EPS bearers / PDP contexts will not be
///   used.
/// - **TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2** - After the PDP
///   context activation, the information text response to the read command
///   provides the configuration negotiated with the network (similarly to
///   +CGTFTRDP and +CGCONTRDP AT commands).
/// - **TOBY-L4 / LARA-R2 / TOBY-R2** - The read command shows PDP contexts/EPS
///   bearers defined by IMS, BIP and OMA-DM internal clients but they cannot be
///   defined, modified or undefined with the set command. It is forbidden to
///   define a PDP context having the same APN used by the IMS internal client,
///   e.g. on TOBY-R2 and LARA-R202 / LARA-R203 / LARA-R211 the APN shall be
///   different from "ims" or "IMS" and on LARA-R204 the APN shall be different
///   from "VZWIMS" or "IMS".
/// - **LARA-R204** - In Verizon Configuration and when attached to Roaming PLMN
///   the Class 3 APN will be defined with <PDP_ type>=IPv4-only at <cid>=1, as
///   per Verizon specifications. Such EPS attach bearer shall then be used for
///   data connectivity. This is not valid on LARA-R204-02B-00,
///   LARA-R204-02B-01.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGDCONT", NoResponse)]
pub struct SetPDPContextDefinition<'a> {
    #[at_arg(position = 0)]
    pub cid: ContextId,
    #[at_arg(position = 1, len = 6)]
    pub pdp_type: &'a str,
    #[at_arg(position = 2, len = 99)]
    pub apn: &'a str,
}

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
    pub profile_id: ProfileId,
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
    pub profile_id: ProfileId,
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
#[at_cmd(
    "+UPSDA",
    NoResponse,
    attempts = 1,
    timeout_ms = 180000,
    abortable = true
)]
pub struct SetPacketSwitchedAction {
    #[at_arg(position = 0)]
    pub profile_id: ProfileId,
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
    pub profile_id: ProfileId,
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
#[at_cmd(
    "+CGATT",
    NoResponse,
    attempts = 1,
    timeout_ms = 180000,
    abortable = true
)]
pub struct SetGPRSAttached {
    #[at_arg(position = 0)]
    pub state: GPRSAttachedState,
}

/// 18.14 Read GPRS attach or detach +CGATT
#[derive(Clone, AtatCmd)]
#[at_cmd(
    "+CGATT?",
    GPRSAttached,
    attempts = 1,
    timeout_ms = 10000,
    abortable = true
)]
pub struct GetGPRSAttached;

/// 18.16 PDP context activate or deactivate +CGACT
///
/// Activates or deactivates the specified PDP context. After the command, the
/// MT remains in AT command mode. If any context is already in the requested
/// state, the state for the context remains unchanged. If the required action
/// cannot succeed, an error result code is returned. If the MT is not GPRS
/// attached when the activation of a PDP context is required, the MT first
/// performs a GPRS attach and then attempts to activate the specified context.
///
/// The maximum expected response time is different whenever the activation or
/// the deactivation of a PDP context is performed (150 s and 40 s
/// respectively). **TOBY-L4 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 /
/// SARA-G4 / SARA-G3 / LEON-G1**
/// - The command can be aborted if a character is sent to the DCE during the
///   command execution: if a PDP context activation on a specific <cid> was
///   requested, the PDP context deactivation is performed; if a multiple PDP
///   context activation was requested, it is aborted after the pending PDP
///   context activation has finished.
/// - The deactivation action is carried out even if the command is aborted.
///
/// **Notes:**
/// - **TOBY-L4 / LARA-R2 / TOBY-R2** - After having aborted the PDP context
///   activation, the command line is not immediately returned.
/// - **SARA-U2 / LISA-U2** - After having aborted the PDP context activation,
///   the command line is immediately returned but the procedure to activate the
///   context is still running and will be completed.
///
/// **LARA-R2 / TOBY-R2**
/// - The read command shows PDP contexts/EPS bearers defined by IMS, BIP and
///   OMA-DM internal clients but they cannot be activated or deactivated.
///
/// **TOBY-L4 / LARA-R2 / TOBY-R2**
/// - The usage of AT+CGACT=0 without specifying the <cid> parameter is
///   deprecated because it can deactivate also PDP contexts / EPS bearer used
///   by internal clients e.g. BIP and OMA-DM.
///
/// **TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 /
/// LISA-U1 / SARA-G4 / SARA-G3 / LEON-G1**
/// - AT+CGACT command (both in successful and unsuccessful case) triggers
///   signalling attempts whose number is internally counted by the SW and
///   limited based on MNO specific thresholds. The AT&T RPM feature (see also
///   the +URPM AT command) and the Verizon configuration (see the +UMNOCONF AT
///   command) might cause the AT command to return an error result code when
///   the maximum number of attempts has been reached. In these cases, the
///   command might become available again after a while.
#[derive(Clone, AtatCmd)]
#[at_cmd(
    "+CGACT",
    NoResponse,
    attempts = 1,
    timeout_ms = 150000,
    abortable = true
)]
pub struct SetPDPContextState {
    #[at_arg(position = 0)]
    pub status: PDPContextStatus,
    #[at_arg(position = 1)]
    pub cid: Option<ContextId>,
}

/// 18.14 Read PDP context state +CGACT
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGACT?", heapless::Vec<PDPContextState, 7>, attempts = 1, timeout_ms = 150000, abortable = true)]
pub struct GetPDPContextState;

/// 18.21 Enter PPP state/GPRS dial-up D*
///
/// The V.24 dial command "D", similar to the command with the syntax
/// AT+CGDATA="PPP",<cid>, causes the MT to perform the necessary actions to
/// establish the communication between the DTE and the external PDP network
/// through the PPP protocol. This can include performing a PS attach and, if
/// the PPP server on the DTE side starts communication, PDP context activation
/// on the specified PDP context identifier (if not already requested by means
/// of +CGATT and +CGACT commands).
///
/// If the command is accepted and the preliminary PS procedures have succeeded,
/// the "CONNECT" intermediate result code is returned, the MT enters the
/// V.25ter online data state and the PPP L2 protocol between the MT and the DTE
/// is started.
#[derive(Clone, AtatCmd)]
#[at_cmd(
    "D*99***",
    NoResponse,
    value_sep = false,
    timeout_ms = 180000,
    abortable = true,
    termination = "#\r\n"
)]
pub struct EnterPPP {
    #[at_arg(position = 0)]
    pub cid: ContextId,
}

/// 18.26 Packet switched event reporting +CGEREP
///
/// Configures sending of URCs from MT to the DTE, in case of certain events
/// occurring in the packet switched MT or the network. By means of the <mode>
/// parameter, it is possible to control the processing of the URCs codes
/// specified within this command. The <bfr> parameter allows to control the
/// effect on buffered codes when the <mode> parameter is set to 1 (discard URCs
/// when V.24 link is reserved) or 2 (buffer URCs in the MT when link reserved
/// and flush them to the DTE when the link becomes available).
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGEREP", NoResponse)]
pub struct SetPacketSwitchedEventReporting {
    #[at_arg(position = 0)]
    pub mode: PSEventReportingMode,
    #[at_arg(position = 1)]
    pub bfr: Option<u8>,
}

/// 18.27 GPRS network registration status +CGREG
///
/// Configures the GPRS network registration information. Depending on the <n>
/// parameter value, a URC can be issued:
/// - +CGREG: <stat> if <n>=1 and there is a change in the GPRS network
///   registration status in GERAN/UTRAN
/// - +CGREG: <stat>[,<lac>,<ci>[,<AcT>,<rac>]] if <n>=2 and there is a change
///   of the network cell in GERAN/ UTRAN
///
/// The parameters <lac>, <ci>, <AcT>, <rac> are provided only if available. The
/// read command provides the same information issued by the URC together with
/// the current value of the <n> parameter. The location information elements
/// <lac>, <ci> and <AcT>, if available, are returned only when <n>=2 and the MT
/// is registered with the network.
///
/// **NOTES:**
/// - When <n>=2, in UMTS RAT, unsolicited location information can be received
///   if the network sends the UTRAN INFORMATION MOBILITY message during
///   dedicated connections; in the latter cases the reported <ci> might be not
///   correct because the UE in DCH state cannot read broadcast system
///   information before the change of serving cell. In contrast, in GSM RAT no
///   unsolicited location information is received during a CS connection.
/// - If the GPRS MT also supports circuit mode services in GERAN/UTRAN and/or
///   EPS services in E-UTRAN, the +CREG / +CEREG commands return the
///   registration status and location information for those services.
/// - **SARA-G4** The command setting is stored in the personal profile
///   following the procedure described in the Saving AT commands configuration
///   section.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGREG", NoResponse)]
pub struct SetGPRSNetworkRegistrationStatus {
    #[at_arg(position = 0)]
    pub n: GPRSNetworkRegistrationUrcConfig,
}

#[derive(Clone, AtatCmd)]
#[at_cmd("+CGREG?", GPRSNetworkRegistrationStatus)]
pub struct GetGPRSNetworkRegistrationStatus;

/// 18.28 Extended Packet Switched network registration status +UREG
///
/// Reports the network or the device PS (Packet Switched) radio capabilities.
/// When the device is not in connected mode, the command reports the network PS
/// (Packet Switched) radio capabilities of the PLMN where the device is
/// attached to.
///
/// When the device is in connected mode, the command reports the PS radio
/// capabilities the device has been configured.
///
/// The set command enables / disables the URC +UREG, generated whenever it is
/// enabled and the capabilities change.
///
/// The read command can be used to query the current PS radio capabilities.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UREG", NoResponse)]
pub struct SetExtendedPSNetworkRegistrationStatus {
    #[at_arg(position = 0)]
    pub n: ExtendedPSNetworkRegistrationUrcConfig,
}

#[derive(Clone, AtatCmd)]
#[at_cmd("+UREG?", ExtendedPSNetworkRegistrationStatus)]
pub struct GetExtendedPSNetworkRegistrationStatus;

/// 18.29 Manual deactivation of a PDP context H
///
/// Deactivates an active PDP context with PPP L2 protocol in online command
/// mode. The MT responds with a final result code. For a detailed description,
/// see the H command description. For additional information about OLCM, see
/// the AT command settings.
///
/// **NOTES**:
/// - In GPRS online command mode, entered by typing the escape sequence "+++"
///   or "~+++" (see &D), the ATH command is needed to terminate the connection.
///   Alternatively, in data transfer mode, DTE originated DTR toggling or PPP
///   disconnection may be used.
#[derive(Clone, AtatCmd)]
#[at_cmd("H", NoResponse)]
pub struct DeactivatePDPContext;

/// 18.36 EPS network registration status +CEREG
///
/// Configures the network registration URC related to EPS domain. The URC
/// assumes a different syntax depending on the network and the <n> parameter:
/// - +CEREG: <stat> when <n>=1 and there is a change in the MT's EPS network
///   registration status in E-UTRAN
/// - +CEREG: <stat>[,[<tac>],[<ci>],[<AcT>]] when <n>=2 and there is a change
///   of the network cell in EUTRAN
/// - +CEREG: <stat>[,[<tac>],[<ci>],[<AcT>][,<cause_type>,<reject_cause>]] when
///   <n>=3 and the value of <stat> changes
/// - +CEREG:
///   <stat>[,[<tac>],[<ci>],[<AcT>][,,[,[<`Assigned_Active_Time`>,[<`Assigned_Periodic_TAU`>]]]]]
///   when <n>=4 if there is a change of the network cell in E-UTRAN
/// - +CEREG:
///   <stat>[,[<tac>],[<ci>],[<AcT>][,[<`cause_type`>],[<`reject_cause`>][,[<`Assigned_Active_Time`>,
///   [<`Assigned_Periodic_TAU`>]]]]] when <n>=5 and the value of <stat> changes
///
/// The parameters <AcT>, <tac>, <`rac_or_mme`>, <ci>, <`cause_type`>,
/// <`reject_cause`>, <`Assigned_Active_Time`> and <`Assigned_Periodic_TAU`> are
/// provided only if available.
///
/// The read command returns always at least the mode configuration (<n>), the
/// EPS registration status (<stat>). The location parameters <tac>,
/// <`rac_or_mme`>, <ci> and <AcT>, if available, are returned only when <n>=2,
/// <n>=3, <n>=4 or <n>=5 and the MT is registered with the network. The
/// parameters <`cause_type`>, <reject_ cause>, if available, are returned when
/// <n>=3 or <n>=5. The PSM related parameter <`Assigned_Active`_ Time> is
/// returned only when <n>=4 or <n>=5, the MT is registered with the network and
/// PSM is granted by the network. The <`Assigned_Periodic_TAU`> parameter is
/// returned only if when <n>=4 or <n>=5, the MT is registered with the network,
/// PSM is granted by the network and an extended periodic TAU value (`T3412_ext`)
/// is assigned.
///
/// **NOTES:**
/// - **TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2** If the EPS MT in
///   GERAN/UTRAN/E-UTRAN also supports circuit mode services and/or GPRS
///   services, the +CREG / +CGREG set and read command result codes apply to
///   the registration status and location information for those services.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CEREG", NoResponse)]
pub struct SetEPSNetworkRegistrationStatus {
    #[at_arg(position = 0)]
    pub n: EPSNetworkRegistrationUrcConfig,
}

#[derive(Clone, AtatCmd)]
#[at_cmd("+CEREG?", EPSNetworkRegistrationStatus)]
pub struct GetEPSNetworkRegistrationStatus;

/// 18.39 Configure the authentication parameters of a PDP/EPS bearer +UAUTHREQ
///
/// Configures the authentication parameters of a defined PDP/EPS bearer. The
/// authentication parameters will be sent during the context activation phase
/// as a protocol configuration options (PCO) information element.
///
/// **NOTES:**
/// - **LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / SARA-G3** - When <`auth_type>=3`
///   is set, AT+CGACT=1,<cid> may trigger at most 3 PDP context activation
///   requests for <cid> to the protocol stack. The first request for <cid> is
///   done with no authentication. If the PDP context activation fails, a second
///   attempt is triggered with PAP authentication. If the second PDP context
///   activation fails, a third attempt is triggered with CHAP authentication.
///   These 3 PDP context activation requests are not to be confused with the
///   effective number of request PDP context activations sent to the network
///   (see the 3GPP TS 24.008 [12]).
/// - **TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 /
///   SARA-G4 / SARA-G3** - The command returns an error result code if the
///   input <cid> is already active or not yet defined.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UAUTHREQ", NoResponse)]
pub struct SetAuthParameters<'a> {
    #[at_arg(position = 0)]
    pub cid: ContextId,
    #[at_arg(position = 1)]
    pub auth_type: AuthenticationType,
    #[at_arg(position = 2, len = 64)]
    pub username: &'a str,
    #[at_arg(position = 3, len = 64)]
    pub password: &'a str,
}
