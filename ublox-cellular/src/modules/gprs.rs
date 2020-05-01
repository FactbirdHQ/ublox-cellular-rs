use embedded_hal::digital::v2::OutputPin;
use heapless::{consts, String};
use no_std_net::{IpAddr, Ipv4Addr};

use crate::{
    command::dns::{self, responses::*},
    command::psn::{self, types::*},
    error::Error,
    GsmClient, State,
};

#[derive(Clone)]
pub struct APNInfo {
    pub apn: String<consts::U99>,
    pub user_name: Option<String<consts::U64>>,
    pub password: Option<String<consts::U64>>,
}

impl APNInfo {
    pub fn new(apn: &str) -> Self {
        APNInfo {
            apn: String::from(apn),
            user_name: None,
            password: None,
        }
    }
}

pub trait GPRS {
    fn attach_gprs(&self, apn_info: APNInfo) -> Result<(), Error>;
    fn detach_gprs(&self) -> Result<(), Error>;
    fn dns_lookup(&self, hostname: &str) -> Result<Ipv4Addr, Error>;
}

impl<C, RST, DTR> GPRS for GsmClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn attach_gprs(&self, apn_info: APNInfo) -> Result<(), Error> {
        // match self.get_state()? {
        //     State::Registered | State::Registering => return Err(Error::_Unknown),
        //     State::Attaching | State::Attached => return Ok(()),
        //     _ => {}
        // };

        self.set_state(State::Attaching)?;

        // Attach GPRS
        self.send_at(&psn::SetGPRSAttached { state: 1 })?;

        // Set APN info
        self.send_at(&psn::SetPacketSwitchedConfig {
            profile_id: 0,
            param: PacketSwitchedParam::APN(apn_info.apn),
        })?;

        // Set auth mode
        self.send_at(&psn::SetPacketSwitchedConfig {
            profile_id: 0,
            param: PacketSwitchedParam::Authentication(AuthenticationType::Auto),
        })?;

        // Set username
        if let Some(user_name) = apn_info.user_name {
            self.send_at(&psn::SetPacketSwitchedConfig {
                profile_id: 0,
                param: PacketSwitchedParam::Username(user_name),
            })?;
        }

        // Set password
        if let Some(password) = apn_info.password {
            self.send_at(&psn::SetPacketSwitchedConfig {
                profile_id: 0,
                param: PacketSwitchedParam::Password(password),
            })?;
        }

        // Set dynamic IP
        self.send_at(&psn::SetPacketSwitchedConfig {
            profile_id: 0,
            param: PacketSwitchedParam::IPAddress(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))),
        })?;

        // Activate IP
        self.send_at(&psn::SetPacketSwitchedAction {
            profile_id: 0,
            action: PacketSwitchedAction::Activate,
        })?;

        // Check profile status
        let psn::responses::PacketSwitchedNetworkData { param_tag, .. } =
            self.send_at(&psn::GetPacketSwitchedNetworkData {
                profile_id: 0,
                param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
            })?;

        if param_tag != 1 {
            self.set_state(State::Deattached)?;
            return Err(Error::Network);
        }
        self.set_state(State::Attached)?;

        Ok(())
    }

    fn detach_gprs(&self) -> Result<(), Error> {
        // Deactivate IP
        self.send_at(&psn::SetPacketSwitchedAction {
            profile_id: 0,
            action: PacketSwitchedAction::Deactivate,
        })?;

        // Detach from network
        self.send_at(&psn::SetGPRSAttached { state: 0 })?;
        self.set_state(State::Deattached)?;

        Ok(())
    }

    fn dns_lookup(&self, hostname: &str) -> Result<Ipv4Addr, Error> {
        let ResolveIpResponse { ip_string } = self.send_at(&dns::ResolveIp {
            domain_string: hostname,
        })?;

        Ok(ip_string.parse().map_err(|_e| Error::Network)?)
    }
}
