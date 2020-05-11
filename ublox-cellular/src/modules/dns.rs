use atat::AtatClient;
use core::fmt::Write;
use embedded_hal::digital::v2::OutputPin;
use embedded_nal::{AddrType, Dns};
use heapless::{consts, String};
use no_std_net::IpAddr;

use crate::{
    command::dns::{self, types::ResolutionType},
    GsmClient,
};

impl<C, RST, DTR> Dns for GsmClient<C, RST, DTR>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    type Error = ();

    fn gethostbyaddr(&self, ip_addr: IpAddr) -> Result<String<consts::U256>, Self::Error> {
        let mut ip_str = String::<consts::U256>::new();
        write!(&mut ip_str, "{}", ip_addr).map_err(|_| ())?;

        let resp = self
            .send_at(&dns::ResolveNameIp {
                resolution_type: ResolutionType::IpToDomainName,
                ip_domain_string: &ip_str,
            })
            .map_err(|_| ())?;

        Ok(String::from(resp.ip_domain_string.as_str()))
    }

    fn gethostbyname(&self, hostname: &str, addr_type: AddrType) -> Result<IpAddr, Self::Error> {
        if addr_type == AddrType::IPv6 {
            return Err(());
        }

        let resp = self
            .send_at(&dns::ResolveNameIp {
                resolution_type: ResolutionType::DomainNameToIp,
                ip_domain_string: hostname,
            })
            .map_err(|_| ())?;

        resp.ip_domain_string.parse().map_err(|_e| ())
    }
}
