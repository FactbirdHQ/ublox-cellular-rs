use super::{socket::SocketSetItem, DataService, Error};
use crate::{
    command::device_data_security::{types::*, *},
    command::ip_transport_layer::{types::*, *},
};
use atat::atat_derive::AtatLen;
use core::convert::TryFrom;
use embedded_nal::tls::{TlsConnect, TlsConnectorConfig};
use embedded_nal::{HostSocketAddr, TcpClientStack};
use heapless::{ArrayLength, Bucket, Pos};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, AtatLen)]
pub struct SecurityProfileId(pub u8);

#[derive(Debug, Clone)]
pub struct TlsConnector {
    profile_id: SecurityProfileId,
}

impl<'a, 'b, CTX> TryFrom<TlsConnectorConfig<'a, &'b CTX>> for TlsConnector {
    type Error = Error;
    fn try_from(_config: TlsConnectorConfig<'a, &'b CTX>) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl<'a, C, N, L> TlsConnect<DataService<'a, C, N, L>> for TlsConnector
where
    C: atat::AtatClient,
    N: ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    type Error = Error;
    fn connect(
        &mut self,
        client: &DataService<'a, C, N, L>,
        socket: &mut <DataService<'a, C, N, L> as TcpClientStack>::TcpSocket,
        remote: HostSocketAddr,
    ) -> nb::Result<(), Self::Error> {
        client
            .network
            .send_internal(
                &SecurityProfileManager {
                    profile_id: self.profile_id,
                    operation: Some(SecurityProfileOperation::CertificateValidationLevel(
                        CertificateValidationLevel::RootCertValidationWithValidityDate,
                    )),
                },
                true,
            )
            .map_err(Self::Error::from)?;

        client
            .network
            .send_internal(
                &SecurityProfileManager {
                    profile_id: self.profile_id,
                    operation: Some(SecurityProfileOperation::CipherSuite(0)),
                },
                true,
            )
            .map_err(Self::Error::from)?;

        client
            .network
            .send_internal(
                &SecurityProfileManager {
                    profile_id: self.profile_id,
                    operation: Some(SecurityProfileOperation::ExpectedServerHostname(
                        remote.addr().hostname().as_ref().unwrap(),
                    )),
                },
                true,
            )
            .map_err(Self::Error::from)?;

        client
            .network
            .send_internal(
                &SetSocketSslState {
                    socket: socket.clone(),
                    ssl_tls_status: SslTlsStatus::Enabled(self.profile_id),
                },
                true,
            )
            .map_err(Self::Error::from)?;

        Ok(())
    }
}
