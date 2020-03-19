//! Responses for Device and data security Commands
use super::types::*;
use atat::atat_derive::AtatResp;
use heapless::{consts, String};

/// 26.1.2 SSL/TLS certificates and private keys manager
#[derive(Clone, PartialEq, AtatResp)]
pub struct SecurityDataImport {
    /// Type of operation
    #[at_arg(position = 0)]
    op_code: SecurityOperation,
    /// Type of the security data
    #[at_arg(position = 1)]
    data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an
    /// existing name is used the data will be overridden.
    ///
    /// **TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / SARA-G4 /
    /// SARA-G3:**
    /// - The maximum length is 200 characters
    #[at_arg(position = 2)]
    internal_name: String<consts::U200>,
    /// MD5 formatted string.
    #[at_arg(position = 3)]
    md5_string: String<consts::U128>,
}
