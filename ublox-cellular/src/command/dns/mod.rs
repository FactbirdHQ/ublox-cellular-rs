//! ### 24 - DNS
//!
//! DNS service requires the user to define and activate a connection profile,
//! either PSD or CSD.
//!
//! **Notes:**
//! - TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 /
//!   SARA-G4 / SARA-G3 / LEON-G1 See +UPSD, +UPSDA and +UPSND AT commands for
//!   establishing a PSD connection.
//! - SARA-G3 / LEON-G1 See +UCSD, +UCSDA and +UCSND AT commands for
//!   establishing a CSD connection.
//!
//! When these command report an error which is not a +CME ERROR, the error
//! class and code is provided through +USOER AT command.

pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use responses::*;
use types::*;

/// 24.1 Resolve name / IP number through DNS +UDNSRN
///
/// Translates a domain name to an IP address or an IP address to a domain name
/// by using an available DNS. There are two available DNSs, primary and
/// secondary. The network usually provides them after a GPRS activation or a
/// CSD establishment. They are automatically used in the resolution process if
/// available. The resolver will use first the primary DNS, otherwise if there
/// is no answer, the second DNS will be involved.
///
/// **Notes:**
/// - TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 /
///   SARA-G4 / SARA-G3 / LEON-G1 The user can replace each network provided DNS
///   by setting its own DNS for a PSD context by means of the +UPSD AT command.
///   If a DNS value different from "0.0.0.0" is provided, the user DNS will
///   replace the correspondent network-provided one. Usage of the network
///   provided DNSs is recommended.
/// - SARA-G3 / LEON-G1 The user can replace each network provided DNS by
///   setting its own DNS for a CSD context by means of the +UCSD AT command. If
///   a DNS value different from "0.0.0.0" is provided, the user DNS will
///   replace the correspondent network-provided one. Usage of the network
///   provided DNSs is recommended.
/// - The DNS resolution timeout depends on the number of DNS servers available
///   to the DNS resolution system. The response time for the DNS resolution is
///   estimated in case 8 servers are used to perform this task.
/// - Pay attention to the DNS setting for the different profiles since the user
///   DNS can be put into action if the corresponding profile is activated (if
///   the user sets a DNS for a profile, and a different profile is activated,
///   the user DNS has no action and the network DNS is used if available).
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDNSRN", ResolveNameIpResponse, attempts = 1, timeout_ms = 120000)]
pub struct ResolveNameIp<'a> {
    #[at_arg(position = 0)]
    pub resolution_type: ResolutionType,
    #[at_arg(position = 1, len = 128)]
    pub ip_domain_string: &'a str,
}
