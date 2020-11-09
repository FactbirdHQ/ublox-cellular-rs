use atat::AtatClient;
use core::fmt::Write;
use embedded_nal::{AddrType, Dns};
use heapless::{consts, ArrayLength, Bucket, Pos, String};
use no_std_net::IpAddr;

use super::{socket::SocketSetItem, DataService, Error};
use crate::{
    command::dns::{self, types::ResolutionType},
};


impl<'a, C, N, L> Dns for DataService<'a, C, N, L>
where
    C: AtatClient,
    N: ArrayLength<Option<SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: ArrayLength<u8>,
{
    type Error = Error;

    fn gethostbyaddr(&self, ip_addr: IpAddr) -> Result<String<consts::U256>, Self::Error> {
        let mut ip_str = String::<consts::U256>::new();
        write!(&mut ip_str, "{}", ip_addr).map_err(|_| Error::BadLength)?;

        let resp = self.network.send_internal(
            &dns::ResolveNameIp {
                resolution_type: ResolutionType::IpToDomainName,
                ip_domain_string: &ip_str,
            },
            true,
        )?;

        Ok(String::from(resp.ip_domain_string.as_str()))
    }

    fn gethostbyname(&self, hostname: &str, addr_type: AddrType) -> Result<IpAddr, Self::Error> {
        if addr_type == AddrType::IPv6 {
            return Err(Error::Dns);
        }

        let resp = self.network.send_internal(
            &dns::ResolveNameIp {
                resolution_type: ResolutionType::DomainNameToIp,
                ip_domain_string: hostname,
            },
            true,
        )?;

        resp.ip_domain_string.parse().map_err(|_e| Error::Dns)
    }
}
