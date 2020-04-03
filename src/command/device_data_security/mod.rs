//! ### 26 - Device and data security
//!
//! SSL/TLS provides a secure connection between two entities using TCP socket
//! for communication (i.e. HTTP/ FTP server and HTTP/FTP client). The SSL/TLS
//! with digital certificates support provides different connection security
//! aspects:
//! - **Server authentication**: use of the server certificate verification
//!   against a specific trusted certificate or a trusted certificates list;
//! - **Client authentication**: use of the client certificate and the
//!   corresponding private key;
//! - **Data security and integrity**: data encryption and Hash Message
//!   Authentication Code (HMAC) generation.
//!
//! The security aspects used in the current connection depend on the SSL/TLS
//! configuration and features supported by the communicating entities. u-blox
//! cellular modules support all the described aspects of SSL/TLS security
//! protocol with these AT commands:
//! - `AT+USECMNG`: import, removal, list and information retrieval of
//!   certificates or private keys;
//! - `AT+USECPRF`: configuration of USECMNG (u-blox SECurity MaNaGement)
//!   profiles used for an SSL/TLS connection.
//!
//! The USECMNG provides a default SSL/TLS profile which cannot be modified. The
//! default USECMNG profile provides the following SSL/TLS settings:
//!
//! | **Setting**                               | **Value**     | **Meaning**                                                                     |
//! |-------------------------------------------|---------------|---------------------------------------------------------------------------------|
//! | Certificates validation level             | Level 0       | The server certificate will not be checked or verified.                         |
//! | Minimum SSL/TLS version                   | Any           | The server can use any of the TLS1.0/TLS1.1/TLS1.2 versions for the connection. |
//! | Cipher suite                              | Automatic     | The cipher suite will be negotiated in the handshake process.                   |
//! | Trusted root certificate internal name    | "" (none)     | No certificate will be used for the server authentication.                      |
//! | Expected server host-name                 | "" (none)     | No server host-name is expected.                                                |
//! | Client certificate internal name          | "" (none)     | No client certificate will be used.                                             |
//! | Client private key internal name          | "" (none)     | No client private key will be used.                                             |
//! | Client private key password               | "" (none)     | No client private key password will be used.                                    |
//! | Pre-shared key                            | "" (none)     | No pre-shared key key password will be used.                                    |
//!
//! **Notes:**
//! - The secure re-negotiation and the SSL/TLS session resumption are currently
//!   not supported, and if mandated by the server the SSL/TLS connection will
//!   fail with an Generic SSL/TLS handshake alert.
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use responses::*;
use types::*;

use super::NoResponse;

/// 26.1.2 SSL/TLS certificates and private keys manager +USECMNG
///
/// Manages the X.509 certificates and private keys with the following
/// functionalities:
/// - Import of certificates and private keys
/// - List and information retrieval of imported certificates and private keys
/// - Removal of certificates and private keys
/// - MD5 calculation of imported certificate or private key
///
/// The number and the format of the certificates and the private keys accepted
/// depend on the module series:
/// - TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / SARA-G4 /
///   SARA-G3 - certificates and private keys both in DER (Distinguished
///   Encoding Rules) and in PEM (Privacy-Enhanced Mail) format are accepted. If
///   the provided format is PEM, the imported certificate or private key will
///   be automatically converted in DER format for the internal storage. It is
///   also possible to validate certificates and private keys. Up to 16
///   certificates or private keys can be imported.
///
/// **Notes:**
/// - The certificates and private keys are kept in DER format and are not
///   retrievable (i.e. cannot be downloaded from the module); for data
///   validation purposes an MD5 hash string of the stored certificate or
///   private key (stored in DER format) can be retrieved.
/// - Data for certificate or private key import can be provided with a stream
///   of byte similar to `+UDWNFILE` or from a file stored on the FS.
#[derive(Clone, AtatCmd)]
#[at_cmd("+USECMNG=0,", NoResponse, value_sep = false)]
pub struct PrepareSecurityDataImport<'a> {
    /// Type of the security data
    #[at_arg(position = 0)]
    pub data_type: SecurityDataType,
    /// Unique identifier of an imported certificate or private key. If an
    /// existing name is used the data will be overridden.
    ///
    /// **TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / SARA-G4 /
    /// SARA-G3:**
    /// - The maximum length is 200 characters
    #[at_arg(position = 1)]
    pub internal_name: &'a str,
    /// Size in bytes of a certificate or private key being imported.
    ///
    /// **TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / SARA-G4 /
    /// SARA-G3:**
    /// - The maximum allowed size is 8192 bytes.
    #[at_arg(position = 2)]
    pub data_size: usize,
    /// Decryption password; applicable only for PKCS8 encrypted client private
    /// keys.
    ///
    /// The maximum length is 128 characters.
    #[at_arg(position = 3)]
    pub password: Option<&'a str>,
}

#[derive(Clone, AtatCmd)]
#[at_cmd(
    "",
    SecurityDataImport,
    value_sep = false,
    cmd_prefix = "",
    termination = "",
    force_receive_state = true
)]
pub struct SendSecurityDataImport<'a> {
    #[at_arg(position = 0)]
    pub data: serde_at::ser::Bytes<'a>,
}

/// 26.1.3 SSL/TLS security layer profile manager +USECPRF
///
/// Manages security profiles for the configuration of the following SSL/TLS
/// connections properties:
/// - Certificate validation level:
///     * Level 0: no certificate validation; the server certificate will not be
///       checked or verified. No additional certificates are needed.
///     * Level 1: certificate validation against a specific or a list of
///       imported trusted root certificates.
///     * Level 2: certificate validation with an additional URL integrity check
///       (the server certificate common name must match the server hostname).
///     * Level 3: certificate validation with an additional check on the
///       certificate validity date. CA certificates should be imported with the
///       +USECMNG AT command
/// - Minimum SSL/TLS version to be used:
///     * Any
///     * TLS 1.0
///     * TLS 1.1
///     * TLS 1.2
/// - Exact cipher suite to be used (the cipher suite register of Internet
///   Assigned Numbers Authority (IANA) is provided in brackets):
///     * (0x002f) TLS_RSA_WITH_AES_128_CBC_SHA
///     * (0x003C) TLS_RSA_WITH_AES_128_CBC_SHA256
///     * (0x0035) TLS_RSA_WITH_AES_256_CBC_SHA
///     * (0x003D) TLS_RSA_WITH_AES_256_CBC_SHA256
///     * (0x000a) TLS_RSA_WITH_3DES_EDE_CBC_SHA
///     * (0x008c) TLS_PSK_WITH_AES_128_CBC_SHA
///     * (0x008d) TLS_PSK_WITH_AES_256_CBC_SHA
///     * (0x008b) TLS_PSK_WITH_3DES_EDE_CBC_SHA
///     * (0x0094) TLS_RSA_PSK_WITH_AES_128_CBC_SHA
///     * (0x0095) TLS_RSA_PSK_WITH_AES_256_CBC_SHA
///     * (0x0093) TLS_RSA_PSK_WITH_3DES_EDE_CBC_SHA
///     * (0x00ae) TLS_PSK_WITH_AES_128_CBC_SHA256
///     * (0x00af) TLS_PSK_WITH_AES_256_CBC_SHA384
///     * (0x00b6) TLS_RSA_PSK_WITH_AES_128_CBC_SHA256
///     * (0x00b7) TLS_RSA_PSK_WITH_AES_256_CBC_SHA384 See Table 84 for the
///       applicability of cipher suite depending on the module series.
/// - Additional cipher suite to be used with IANA enumeration set command, see
///   Syntax description and Table 85
/// - Certificate to be used for server and mutual authentication:
///     * The trusted root certificate. The CA certificate should be imported
///       with the AT+USECMNG command.
///     * The client certificate that should be imported with the AT+USECMNG
///       command.
///     * The client private key that should be imported with the AT+USECMNG
///       command.
/// - Expected server hostname, when using certificate validation level 2 or 3.
/// - Password for the client private key, if it is password protected.
/// - Pre-shared key used for connection. Defines a pre-shared key and key-name
///   (PSK), when a TLS_PSK_* cipher suite is used.
/// - SNI (Server Name Indication). SNI is a feature of SSL/TLS which uses an
///   additional SSL/TLS extension header to specify the server name to which
///   the client is connecting to. The extension was introduced to support the
///   certificate handling used with virtual hosting provided by the various
///   SSL/TLS enabled servers mostly in cloud based infrastructures. With the
///   SNI a server has the opportunity to present a different server certificate
///   (or/and whole SSL/TLS configuration) based on the host indicated by the
///   SNI extension.
/// - TLS session resumption. The session resumption feature allows to reuse the
///   secure session data in order to reestablish a SSL/TLS secure session.
///   Since the secure session data are available, the SSL/TLS handshake is not
///   performed during the session resumption. Once the session resumption
///   feature is enabled, the session resumption type (provided by the server)
///   and the secure session data (negotiated during the SSL/ TLS handshake) are
///   displayed via +USECPRF URC message. The session resumption feature
///   configuration and secure session data are not stored in the NVM, hence the
///   session resumption may be performed until power cycle.
///
/// **Notes:**
/// - To set all the parameters in security profile, a set command for each
///   <op_code> needs to be issued (e.g. certificate validation level, minimum
///   SSL/TLS version, ...).
/// - To reset (set to factory-programmed value) all the parameters of a
///   specific security profile, issue the AT +USECPRF=<profile_id> command.
#[derive(Clone, AtatCmd)]
#[at_cmd("+USECPRF", NoResponse)]
pub struct SecurityProfileManager {
    /// USECMNG security profile identifier, in range 0-4; if it is not followed
    /// by other parameters the profile settings will be reset (set to
    /// factory-programmed value)
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub op_code: SecurityProfileOperation,
    #[at_arg(position = 2)]
    pub arg: u8, // pub op_code: SecurityProfileOperation
}

#[derive(Clone, AtatCmd)]
#[at_cmd("+USECPRF", NoResponse)]
pub struct SecurityProfileManagerString<'a> {
    /// USECMNG security profile identifier, in range 0-4; if it is not followed
    /// by other parameters the profile settings will be reset (set to
    /// factory-programmed value)
    #[at_arg(position = 0)]
    pub profile_id: u8,
    #[at_arg(position = 1)]
    pub op_code: SecurityProfileOperation,
    #[at_arg(position = 2)]
    pub arg: &'a str, // pub op_code: SecurityProfileOperation
}
