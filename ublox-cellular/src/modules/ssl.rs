use embedded_hal::digital::v2::OutputPin;

use crate::{
    command::device_data_security::{types::*, *},
    command::ip_transport_layer::{types::*, *},
    error::Error,
    socket::SocketHandle,
    GsmClient,
};

pub trait SSL {
    fn import_certificate(
        &self,
        profile_id: u8,
        name: &str,
        certificate: &[u8],
    ) -> Result<(), Error>;
    fn import_root_ca(&self, profile_id: u8, name: &str, root_ca: &[u8]) -> Result<(), Error>;
    fn import_private_key(
        &self,
        profile_id: u8,
        name: &str,
        private_key: &[u8],
        password: Option<&str>,
    ) -> Result<(), Error>;
    fn enable_ssl(&self, socket: SocketHandle, profile_id: u8) -> Result<(), Error>;
}

impl<C, RST, DTR> SSL for GsmClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn import_certificate(
        &self,
        profile_id: u8,
        name: &str,
        certificate: &[u8],
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(&PrepareSecurityDataImport {
            data_type: SecurityDataType::ClientCertificate,
            data_size: certificate.len(),
            internal_name: name,
            password: None,
        })?;

        self.send_at(&SendSecurityDataImport {
            data: serde_at::ser::Bytes(certificate),
        })?;

        self.send_at(&SecurityProfileManager {
            profile_id,
            operation: Some(SecurityProfileOperation::ClientCertificateInternalName(
                name,
            )),
        })?;

        Ok(())
    }

    fn import_root_ca(&self, profile_id: u8, name: &str, root_ca: &[u8]) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(&PrepareSecurityDataImport {
            data_type: SecurityDataType::TrustedRootCA,
            data_size: root_ca.len(),
            internal_name: name,
            password: None,
        })?;

        self.send_at(&SendSecurityDataImport {
            data: serde_at::ser::Bytes(root_ca),
        })?;

        self.send_at(&SecurityProfileManager {
            profile_id,
            operation: Some(SecurityProfileOperation::TrustedRootCertificateInternalName(name)),
        })?;

        Ok(())
    }

    fn import_private_key(
        &self,
        profile_id: u8,
        name: &str,
        private_key: &[u8],
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
            data: serde_at::ser::Bytes(private_key),
        })?;

        self.send_at(&SecurityProfileManager {
            profile_id,
            operation: Some(SecurityProfileOperation::ClientPrivateKeyInternalName(name)),
        })?;

        Ok(())
    }

    fn enable_ssl(&self, socket: SocketHandle, profile_id: u8) -> Result<(), Error> {
        self.send_at(&SecurityProfileManager {
            profile_id,
            operation: Some(SecurityProfileOperation::CertificateValidationLevel(
                CertificateValidationLevel::RootCertValidationWithValidityDate,
            )),
        })?;

        self.send_at(&SecurityProfileManager {
            profile_id,
            operation: Some(SecurityProfileOperation::CipherSuite(0)),
        })?;

        self.send_at(&SecurityProfileManager {
            profile_id,
            operation: Some(SecurityProfileOperation::ExpectedServerHostname(
                "a3f8k0ccx04zas.iot.eu-west-1.amazonaws.com",
            )),
        })?;

        self.send_at(&SetSocketSslState {
            socket,
            ssl_tls_status: SslTlsStatus::Enabled,
            profile_id, // ssl_tls_status: SslTlsStatus::Enabled(profile)
        })?;

        Ok(())
    }
}

// impl<C, RST, DTR> Tls for GsmClient<C, RST, DTR>
// where
//     C: atat::AtatClient,
//     RST: OutputPin,
//     DTR: OutputPin,
// {
//     fn connect_tls(
//         &self,
//         socket: <Self as TcpStack>::TcpSocket,
//         connector: TlsConnector,
//         domain: &str,
//         port: u16,
//     ) -> Result<TlsSocket<<Self as TcpStack>::TcpSocket>, ()> {
//         let profile = 0u8;

//         if let Some(root_ca) = connector.root_certificate() {
//             self.import_root_ca(profile)
//         }

//         if let Some(ident) = connector.identity() {

//         }

//         self.enable_ssl(socket, 0).map_err(|_| ())?;
//         Ok(TlsSocket::new(socket))
//     }
// }
