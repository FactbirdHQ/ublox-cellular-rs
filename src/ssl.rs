use atat::prelude::*;
use embedded_hal::digital::v2::OutputPin;
use heapless::{consts, String};
use no_std_net::{IpAddr, Ipv4Addr};

use crate::{
    error::Error,
    socket::{AnySocket, SocketHandle},
    GSMClient,
};

pub trait SSL {
    fn upgrade_socket(&self, socket: &SocketHandle) -> Result<(), Error>;
}

impl<C, RST, DTR> SSL for GSMClient<C, RST, DTR>
where
    C: atat::ATATInterface,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn upgrade_socket(&self, socket: &SocketHandle) -> Result<(), Error> {
        Ok(())
    }
}
