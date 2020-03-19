//! Argument and parameter types used by Device and data security Commands and Responses

use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;
// use atat::atat_derive::AtatEnum;

/// Type of operation
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SecurityOperation {
    /// 0: import a certificate or a private key (data provided by the stream of byte)
    ImportStream = 0,
    /// 1: import a certificate or a private key (data provided from a file on FS)
    ImportFS = 1,
    /// 2: remove an imported certificate or private key
    Remove = 2,
    /// 3: list imported certificates or private keys
    List = 3,
    /// 4: retrieve the MD5 of an imported certificate or private key
    RetrieveMD5 = 4,
}

/// Type of the security data
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SecurityDataType {
    /// 0: trusted root CA (certificate authority) certificate
    TrustedRootCA = 0,
    /// 1: client certificate
    ClientCertificate = 1,
    /// 2: client private key
    ClientPrivateKey = 2,
    /// 3: RFU
    RFU = 3,
    /// 4: signature verification certificate
    SignatureVerificationCertificate = 4,
    /// 5: signature verification public key
    SignatureVerificationPublicKey = 5,
}

/// certificate validation level
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum CertificateValidationLevel {
    /// * 0 (factory-programmed value): level 0 - No validation; the server
    ///   certificate will not be checked or verified. The server in this case
    ///   is not authenticated.
    NoValidation = 0,
    /// * 1: level 1 - Root certificate validation without URL integrity check.
    ///   The server certificate will be verified with a specific trusted
    ///   certificates or with each of the imported trusted root certificates.
    RootCertValidationWithoutIntegrity = 1,
    /// * 2: level 2 - Root certificate validation with URL integrity check.
    ///   Level 1 validation with an additional URL integrity check. evel 3 -
    ///   Root certificate validation with check of certificate validity date.
    ///   Level 2 validation with an additional check of certificate validity
    ///   date.
    RootCertValidationWithIntegrity = 2
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SecurityProfileOperation {
    /// - 0: certificate validation level;
    CertificateValidationLevel = 0,
    /// - 1: SSL/TLS version to use; allowed values for <param_val1>:
    ///     * 0 (factory-programmed value): any; server can use any version for
    ///       the connection.
    ///     * 1: TLSv1.0; connection allowed only to TLS/SSL servers which
    ///       support TLSv1.0
    ///     * 2: TLSv1.1; connection allowed only to TLS/SSL servers which
    ///       support TLSv1.1
    ///     * 3: TLSv1.2; connection allowed only to TLS/SSL servers which
    ///       support TLSv1.2
    SslTslVersion = 1,
    /// - 2: cipher suite; allowed values for <param_val1> define which cipher
    ///   suite will be used:
    ///     * 0 (factory-programmed value): (0x0000) Automatic the cipher suite
    ///       will be negotiated in the handshake process
    ///     * 1: (0x002f) TLS_RSA_WITH_AES_128_CBC_SHA
    ///     * 2: (0x003C) TLS_RSA_WITH_AES_128_CBC_SHA256
    ///     * 3: (0x0035) TLS_RSA_WITH_AES_256_CBC_SHA
    ///     * 4: (0x003D) TLS_RSA_WITH_AES_256_CBC_SHA256
    ///     * 5: (0x000a) TLS_RSA_WITH_3DES_EDE_CBC_SHA
    ///     * 6: (0x008c) TLS_PSK_WITH_AES_128_CBC_SHA
    ///     * 7: (0x008d) TLS_PSK_WITH_AES_256_CBC_SHA
    ///     * 8: (0x008b) TLS_PSK_WITH_3DES_EDE_CBC_SHA
    ///     * 9: (0x0094) TLS_RSA_PSK_WITH_AES_128_CBC_SHA
    ///     * 10: (0x0095) TLS_RSA_PSK_WITH_AES_256_CBC_SHA
    ///     * 11: (0x0093) TLS_RSA_PSK_WITH_3DES_EDE_CBC_SHA
    ///     * 12: (0x00ae) TLS_PSK_WITH_AES_128_CBC_SHA256
    ///     * 13: (0x00af) TLS_PSK_WITH_AES_256_CBC_SHA384
    ///     * 14: (0x00b6) TLS_RSA_PSK_WITH_AES_128_CBC_SHA256
    ///     * 15: (0x00b7) TLS_RSA_PSK_WITH_AES_256_CBC_SHA384
    ///     * 99: cipher suite selection using IANA enumeration, <byte_1> and
    ///       <byte_2> are strings containing the 2 bytes that compose the IANA
    ///       enumeration, see Table 85.
    CipherSuite = 2,
    /// - 3: trusted root certificate internal name;
    ///     * <param_val1> (string) is the internal name identifying a trusted
    ///       root certificate; the maximum length is 200 characters. The
    ///       factory-programmed value is an empty string.
    TrustedRootCertificateInternalName = 3,
    /// - 4: expected server hostname;
    ///     * <param_val1> (string) is the hostname of the server, used when
    ///       certificate validation level is set to Level 2; the maximum length
    ///       is 256 characters. The factory-programmed value is an empty
    ///       string.
    ExpectedServerHostname = 4,
    /// - 5: client certificate internal name;
    ///     * <param_val1> (string) is the internal name identifying a client
    ///       certificate to be sent to the server; the maximum length is 200
    ///       characters. The factory-programmed value is an empty string.
    ClientCertificateInternalName = 5,
    /// - 6: client private key internal name;
    ///     * <param_val1> (string) is the internal name identifying a private
    ///       key to be used; the maximum length is 200 characters. The
    ///       factory-programmed value is an empty string.
    ClientPrivateKeyInternalName = 6,
    /// - 7: client private key password;
    ///     * <param_val1> (string) is the password for the client private key
    ///       if it is password protected; the maximum length is 128 characters.
    ///       The factory-programmed value is an empty string.
    ClientPrivateKeyPassword = 7,
    /// - 8: pre-shared key;
    ///     * <preshared_key> (string) is the pre-shared key used for
    ///       connection; the factoryprogrammed value is an empty string. The
    ///       accepted string type and length depends on the <string_type>
    ///       value.
    ///     * <string_type> (number) defines the type and the maximum length of
    ///       the <preshared_key> string. Allowed values for <string_type>:
    ///         - 0 (default value): <preshared_key> is an ASCII string and its
    ///           maximum length is 64 characters
    ///         - 1: <preshared_key> is an hexadecimal string and its maximum
    ///           length is 128 characters
    PresharedKey = 8,
    ///  - 9: pre-shared key identity;
    ///     * <preshared_key_id> (string) is the pre-shared key identity used
    ///       for connection; the factoryprogrammed value is an empty string.
    ///       The accepted string type and length depends on the <string_type>
    ///       value.
    ///     * <string_type> (number) defines the type of the <preshared_key_id>
    ///       string. Allowed values for <string_type>:
    ///         - 0 (default value): <preshared_key_id> is an ASCII string and
    ///           its maximum length is 128 characters
    ///         - 1: <preshared_key_id> is an hexadecimal string and its maximum
    ///           length is 256 characters
    PresharedKeyIdentity = 9,
    ///  - 10: SNI (Server Name Indication);
    ///     * <param_val1> (string) value for the additional negotiation header
    ///       SNI (Server Name Indication) used in SSL/TLS connection
    ///       negotiation; the maximum length is 128 characters. The
    ///       factory-programmed value is an empty string.
    ServerNameIndication = 10,
    ///  - 11: PSK key and PSK key identity generated by RoT (Root of trust);
    ///    allowed values for <param_ val1>:
    ///     * 0 (factory-programmed value): OFF - The PSK and PSK key ID are NOT
    ///       generated by RoT
    ///     * 1: ON - The PSK and PSK key ID are generated by RoT in the process
    ///       of SSL/TLS connection negotiation
    PskKey = 11,
    ///  - 12: server certificate pinning;
    ///     * <server_certificate> (string) internal name identifying a
    ///       certificate configured to be used for server certificate pinning;
    ///       the maximum length is 200 characters. The factoryprogrammed value
    ///       is an empty string.
    ///     * <pinning_level> defines the certificate pinning information level.
    ///       Allowed values for <pinning_level>
    ///         - 0: pinning based on information comparison of received and
    ///           configured certificate public key
    ///         - 1: pinning based on binary comparison of received and
    ///           configured certificate public key
    ///         - 2: pinning based on binary comparison of received and
    ///           configured certificate
    ServerCertificatePinning = 12,
    ///  - 13: TLS session resumption;
    ///     * <tag> (number) configures the TLS session resumption. Allowed
    ///       values:
    ///         - 0: session resumption status
    ///             * <param_val1> (number) configures the session resumption
    ///               status. Allowed values: o 0 (factory-programmed value):
    ///               disabled o 1: enabled
    ///         - 1: session resumption type
    ///             * <param_val1> (number) configures the session resumption
    ///               type. Allowed values: o 0: session ID
    ///         - 2: session resumption data for <param_val1>=0 (session
    ///               resumption type is session ID)
    ///             * <param_val1> (string): base64 encoded session ID value.
    ///               The maximum length is 48 characters
    ///             * <param_val2> (string): base64 encoded session master key.
    ///               The maximum length is 64 characters
    TlsSessionResumption = 13,
}
