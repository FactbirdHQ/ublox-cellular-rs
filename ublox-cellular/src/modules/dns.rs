use atat::AtatClient;
use core::fmt::Write;
use embedded_hal::{blocking::delay::DelayMs, digital::{OutputPin, InputPin}};
use embedded_nal::{AddrType, Dns};
use heapless::{consts, ArrayLength, Bucket, Pos, PowerOfTwo, String};
use no_std_net::IpAddr;

use crate::{
    command::dns::{self, types::ResolutionType},
    error::Error,
    GsmClient,
};

impl<C, DLY, N, L, RST, DTR, PWR, VINT> Dns for GsmClient<C, DLY, N, L, RST, DTR, PWR, VINT>
where
    C: AtatClient,
    DLY: DelayMs<u32>,
    RST: OutputPin,
    PWR: OutputPin,
    DTR: OutputPin,
    VINT: InputPin,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>
        + PowerOfTwo,
    L: ArrayLength<u8>,
{
    type Error = Error;

    fn gethostbyaddr(&self, ip_addr: IpAddr) -> Result<String<consts::U256>, Self::Error> {
        let mut ip_str = String::<consts::U256>::new();
        write!(&mut ip_str, "{}", ip_addr).map_err(|_| Error::BadLength)?;

        let resp = self.send_at(&dns::ResolveNameIp {
            resolution_type: ResolutionType::IpToDomainName,
            ip_domain_string: &ip_str,
        })?;

        Ok(String::from(resp.ip_domain_string.as_str()))
    }

    fn gethostbyname(&self, hostname: &str, addr_type: AddrType) -> Result<IpAddr, Self::Error> {
        if addr_type == AddrType::IPv6 {
            return Err(Error::Dns);
        }

        let resp = self.send_at(&dns::ResolveNameIp {
            resolution_type: ResolutionType::DomainNameToIp,
            ip_domain_string: hostname,
        })?;

        resp.ip_domain_string.parse().map_err(|_e| Error::Dns)
    }
}
