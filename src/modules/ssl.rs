use embedded_hal::digital::v2::OutputPin;

use crate::{
    command::device_data_security::{types::*, *},
    command::ip_transport_layer::{types::*, *},
    error::Error,
    socket::SocketHandle,
    GSMClient,
};

pub trait SSL {
    fn import_certificate(
        &self,
        profile_id: u8,
        name: &str,
        certificate: &str,
    ) -> Result<(), Error>;
    fn import_root_ca(&self, profile_id: u8, name: &str, root_ca: &str) -> Result<(), Error>;
    fn import_private_key(
        &self,
        profile_id: u8,
        name: &str,
        private_key: &str,
        password: Option<&str>,
    ) -> Result<(), Error>;
    fn enable_ssl(&self, socket: SocketHandle, profile_id: u8) -> Result<(), Error>;
}

impl<C, RST, DTR> SSL for GSMClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn import_certificate(
        &self,
        profile_id: u8,
        name: &str,
        certificate: &str,
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(&PrepareSecurityDataImport {
            data_type: SecurityDataType::ClientCertificate,
            data_size: certificate.len(),
            internal_name: name,
            password: None,
        })?;

        self.send_at(&SendSecurityDataImport {
            data: serde_at::ser::Bytes(certificate.as_bytes()),
        })?;

        self.send_at(&SecurityProfileManagerString {
            profile_id,
            op_code: SecurityProfileOperation::ClientCertificateInternalName,
            arg: name,
        })?;

        Ok(())
    }

    fn import_root_ca(&self, profile_id: u8, name: &str, root_ca: &str) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(&PrepareSecurityDataImport {
            data_type: SecurityDataType::TrustedRootCA,
            data_size: root_ca.len(),
            internal_name: name,
            password: None,
        })?;

        self.send_at(&SendSecurityDataImport {
            data: serde_at::ser::Bytes(root_ca.as_bytes()),
        })?;

        self.send_at(&SecurityProfileManagerString {
            profile_id,
            op_code: SecurityProfileOperation::TrustedRootCertificateInternalName,
            arg: name,
        })?;

        Ok(())
    }

    fn import_private_key(
        &self,
        profile_id: u8,
        name: &str,
        private_key: &str,
        password: Option<&str>,
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(&PrepareSecurityDataImport {
            data_type: SecurityDataType::ClientPrivateKey,
            data_size: private_key.len(),
            internal_name: name,
            password,
        })?;

        self.send_at(&SendSecurityDataImport {
            data: serde_at::ser::Bytes(private_key.as_bytes()),
        })?;

        self.send_at(&SecurityProfileManagerString {
            profile_id,
            op_code: SecurityProfileOperation::ClientPrivateKeyInternalName,
            arg: name,
        })?;

        Ok(())
    }

    fn enable_ssl(&self, socket: SocketHandle, profile_id: u8) -> Result<(), Error> {
        self.send_at(&SecurityProfileManager {
            profile_id,
            op_code: SecurityProfileOperation::CertificateValidationLevel,
            arg: 3,
        })?;

        self.send_at(&SecurityProfileManager {
            profile_id,
            op_code: SecurityProfileOperation::CipherSuite,
            arg: 0,
        })?;

        self.send_at(&SecurityProfileManagerString {
            profile_id,
            op_code: SecurityProfileOperation::ExpectedServerHostname,
            // Staging:
            arg: "a3f8k0ccx04zas.iot.eu-west-1.amazonaws.com",
            // Playground:
            // arg: "a69ih9fwq4cti.iot.eu-west-1.amazonaws.com",
            // arg: "test.mosquitto.org",
        })?;

        self.send_at(&SetSocketSslState {
            socket,
            ssl_tls_status: SslTlsStatus::Enabled,
            profile_id, // ssl_tls_status: SslTlsStatus::Enabled(profile)
        })?;

        Ok(())
    }
}
