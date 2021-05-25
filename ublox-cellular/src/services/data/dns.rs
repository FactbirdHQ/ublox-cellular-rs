use atat::AtatClient;
use core::fmt::Write;
use embedded_nal::IpAddr;
use embedded_nal::{AddrType, Dns};
use embedded_time::Clock;
use heapless::String;

use super::{DataService, Error};
use crate::command::dns::{self, types::ResolutionType};

impl<'a, C, CLK, const N: usize, const L: usize> Dns for DataService<'a, C, CLK, N, L>
where
    C: AtatClient,
    CLK: Clock,
{
    type Error = Error;

    fn get_host_by_address(&mut self, ip_addr: IpAddr) -> nb::Result<String<256>, Self::Error> {
        let mut ip_str = String::<256>::new();
        write!(&mut ip_str, "{}", ip_addr).map_err(|_| Error::BadLength)?;

        let resp = self
            .network
            .send_internal(
                &dns::ResolveNameIp {
                    resolution_type: ResolutionType::IpToDomainName,
                    ip_domain_string: &ip_str,
                },
                true,
            )
            .map_err(|_| Error::Dns)?;

        Ok(String::from(resp.ip_domain_string.as_str()))
    }

    fn get_host_by_name(
        &mut self,
        hostname: &str,
        addr_type: AddrType,
    ) -> nb::Result<IpAddr, Self::Error> {
        if addr_type == AddrType::IPv6 {
            return Err(nb::Error::Other(Error::Dns));
        }

        let resp = self
            .network
            .send_internal(
                &dns::ResolveNameIp {
                    resolution_type: ResolutionType::DomainNameToIp,
                    ip_domain_string: hostname,
                },
                true,
            )
            .map_err(|_| Error::Dns)?;

        resp.ip_domain_string
            .parse()
            .map_err(|_e| nb::Error::Other(Error::Dns))
    }
}
