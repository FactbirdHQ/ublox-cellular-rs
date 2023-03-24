use atat::{clock::Clock, AtatClient};
use core::fmt::Write;
use embedded_nal::IpAddr;
use embedded_nal::{AddrType, Dns};
use heapless::String;

use super::DataService;
use crate::command::dns::{self, types::ResolutionType};
use ublox_sockets::Error;

impl<'a, C, CLK, const TIMER_HZ: u32, const N: usize, const L: usize> Dns
    for DataService<'a, C, CLK, TIMER_HZ, N, L>
where
    C: AtatClient,
    CLK: Clock<TIMER_HZ>,
{
    type Error = Error;

    fn get_host_by_address(&mut self, ip_addr: IpAddr) -> nb::Result<String<256>, Self::Error> {
        let mut ip_str = String::<256>::new();
        write!(&mut ip_str, "{ip_addr}").map_err(|_| Error::BadLength)?;

        match self.network.send_internal(
            &dns::ResolveNameIp {
                resolution_type: ResolutionType::IpToDomainName,
                ip_domain_string: &ip_str,
            },
            true,
        ) {
            Ok(resp) => Ok(String::from(resp.ip_domain_string.as_str())),
            Err(e) => {
                error!("get_host_by_address failed: {:?}", e);
                Err(nb::Error::Other(Error::Unaddressable))
            }
        }
    }

    fn get_host_by_name(
        &mut self,
        hostname: &str,
        addr_type: AddrType,
    ) -> nb::Result<IpAddr, Self::Error> {
        if addr_type == AddrType::IPv6 {
            return Err(nb::Error::Other(Error::Illegal));
        }

        match self.network.send_internal(
            &dns::ResolveNameIp {
                resolution_type: ResolutionType::DomainNameToIp,
                ip_domain_string: hostname,
            },
            true,
        ) {
            Ok(resp) => resp
                .ip_domain_string
                .parse()
                .map_err(|_e| nb::Error::Other(Error::Illegal)),
            Err(e) => {
                error!("get_host_by_name failed: {:?}", e);
                Err(nb::Error::Other(Error::Unaddressable))
            }
        }
    }
}
