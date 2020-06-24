use atat::AtatClient;
use core::fmt::Write;
use embedded_hal::digital::v2::OutputPin;
use embedded_nal::{AddrType, Dns};
use heapless::{consts, String};
use no_std_net::IpAddr;

use crate::{
    command::dns::{self, types::ResolutionType},
    GsmClient,
    error::Error
};

impl<C, RST, DTR> Dns for GsmClient<C, RST, DTR>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = Error;

    fn gethostbyaddr(&self, ip_addr: IpAddr) -> Result<String<consts::U256>, Self::Error> {
        let mut ip_str = String::<consts::U256>::new();
        write!(&mut ip_str, "{}", ip_addr).map_err(|_| Error::BadLength)?;

        let resp = self
            .send_at(&dns::ResolveNameIp {
                resolution_type: ResolutionType::IpToDomainName,
                ip_domain_string: &ip_str,
            })?;

        Ok(String::from(resp.ip_domain_string.as_str()))
    }

    fn gethostbyname(&self, hostname: &str, addr_type: AddrType) -> Result<IpAddr, Self::Error> {
        if addr_type == AddrType::IPv6 {
            return Err(Error::Dns);
        }

        #[cfg(feature = "logging")]
        log::info!("hostname: {:?}", hostname);

        let resp = self
            .send_at(&dns::ResolveNameIp {
                resolution_type: ResolutionType::DomainNameToIp,
                ip_domain_string: hostname,
            })?;

        resp.ip_domain_string.parse().map_err(|_e| Error::Dns)
    }
}
