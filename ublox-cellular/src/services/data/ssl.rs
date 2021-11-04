use super::{Clock, DataService, Error};
use crate::command::device_data_security::{types::*, *};
use atat::atat_derive::AtatLen;
use heapless::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, AtatLen)]
pub struct SecurityProfileId(pub u8);

pub trait SSL {
    fn import_certificate(
        &mut self,
        profile_id: SecurityProfileId,
        name: &str,
        certificate: &[u8],
    ) -> Result<(), Error>;
    fn import_root_ca(
        &mut self,
        profile_id: SecurityProfileId,
        name: &str,
        root_ca: &[u8],
    ) -> Result<(), Error>;
    fn import_private_key(
        &mut self,
        profile_id: SecurityProfileId,
        name: &str,
        private_key: &[u8],
        password: Option<&str>,
    ) -> Result<(), Error>;
    fn enable_ssl(
        &mut self,
        profile_id: SecurityProfileId,
        server_hostname: &str,
    ) -> Result<(), Error>;
}

impl<'a, C, CLK, const TIMER_HZ: u32, const N: usize, const L: usize> SSL
    for DataService<'a, C, CLK, TIMER_HZ, N, L>
where
    C: atat::AtatClient,
    CLK: Clock<TIMER_HZ>,
{
    fn import_certificate(
        &mut self,
        profile_id: SecurityProfileId,
        name: &str,
        certificate: &[u8],
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.network.send_internal(
            &PrepareSecurityDataImport {
                data_type: SecurityDataType::ClientCertificate,
                data_size: certificate.len(),
                internal_name: name,
                password: None,
            },
            true,
        )?;

        self.network.send_internal(
            &SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(certificate),
            },
            true,
        )?;

        self.network.send_internal(
            &SecurityProfileManager {
                profile_id,
                operation: Some(SecurityProfileOperation::ClientCertificateInternalName(
                    String::from(name),
                )),
            },
            true,
        )?;

        Ok(())
    }

    fn import_root_ca(
        &mut self,
        profile_id: SecurityProfileId,
        name: &str,
        root_ca: &[u8],
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.network.send_internal(
            &PrepareSecurityDataImport {
                data_type: SecurityDataType::TrustedRootCA,
                data_size: root_ca.len(),
                internal_name: name,
                password: None,
            },
            true,
        )?;

        self.network.send_internal(
            &SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(root_ca),
            },
            true,
        )?;

        self.network.send_internal(
            &SecurityProfileManager {
                profile_id,
                operation: Some(
                    SecurityProfileOperation::TrustedRootCertificateInternalName(String::from(
                        name,
                    )),
                ),
            },
            true,
        )?;

        Ok(())
    }

    fn import_private_key(
        &mut self,
        profile_id: SecurityProfileId,
        name: &str,
        private_key: &[u8],
        password: Option<&str>,
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.network.send_internal(
            &PrepareSecurityDataImport {
                data_type: SecurityDataType::ClientPrivateKey,
                data_size: private_key.len(),
                internal_name: name,
                password,
            },
            true,
        )?;

        self.network.send_internal(
            &SendSecurityDataImport {
                data: atat::serde_bytes::Bytes::new(private_key),
            },
            true,
        )?;

        self.network.send_internal(
            &SecurityProfileManager {
                profile_id,
                operation: Some(SecurityProfileOperation::ClientPrivateKeyInternalName(
                    String::from(name),
                )),
            },
            true,
        )?;

        Ok(())
    }

    fn enable_ssl(
        &mut self,
        profile_id: SecurityProfileId,
        server_hostname: &str,
    ) -> Result<(), Error> {
        self.network.send_internal(
            &SecurityProfileManager {
                profile_id,
                operation: Some(SecurityProfileOperation::CertificateValidationLevel(
                    CertificateValidationLevel::RootCertValidationWithValidityDate,
                )),
            },
            true,
        )?;

        self.network.send_internal(
            &SecurityProfileManager {
                profile_id,
                operation: Some(SecurityProfileOperation::CipherSuite(0)),
            },
            true,
        )?;

        self.network.send_internal(
            &SecurityProfileManager {
                profile_id,
                operation: Some(SecurityProfileOperation::ExpectedServerHostname(
                    String::from(server_hostname),
                )),
            },
            true,
        )?;

        Ok(())
    }
}
