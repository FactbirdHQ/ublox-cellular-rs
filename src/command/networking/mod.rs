//! ### 34 - Networking
pub mod responses;
pub mod types;

use super::NoResponse;
use atat::atat_derive::AtatCmd;

use responses::EmbeddedPortFiltering;
use types::EmbeddedPortFilteringMode;

/// 34.4 Configure port filtering for embedded applications +UEMBPF
///
/// Enables/disables port filtering to allow IP data traffic from embedded
/// applications when a dial-up connection is active. When enabled, the
/// application will pick source port inside the configured range and the
/// incoming traffic to those ports will be directed to embedded application
/// instead of PPP DTE.
///
/// **NOTE:**
/// - Each set command overwrites the previous configuration. Only one port
///   range can be configured.
/// - When set command with <mode>=0 is issued, the parameter <port_range> shall
/// not be inserted otherwise the command will return an error result code.
/// - If <mode>=0 is configured, the read command will not return any range, but
/// only +UEMBPF: 0 as information text response.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UEMBPF?", EmbeddedPortFiltering)]
pub struct GetEmbeddedPortFiltering;

/// 34.4 Configure port filtering for embedded applications +UEMBPF
#[derive(Clone, AtatCmd)]
#[at_cmd("+UEMBPF", NoResponse)]
pub struct SetEmbeddedPortFiltering {
    #[at_arg(position = 0)]
    pub mode: EmbeddedPortFilteringMode,
}
