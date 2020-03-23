use embedded_hal::digital::v2::OutputPin;
use heapless::{consts, String};
use no_std_net::{IpAddr, Ipv4Addr};

use crate::{
    command::psn::{self, types::*},
    error::Error,
    GSMClient,
    GSMState
};

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
}

impl<C, RST, DTR> GPRS for GSMClient<C, RST, DTR>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    fn attach_gprs(&self, apn_info: APNInfo) -> Result<(), Error> {
        match self.get_state()? {
            GSMState::Registered | GSMState::Registering => return Err(Error::_Unknown),
            GSMState::Attaching | GSMState::Attached => return Ok(()),
            _ => {}
        };

        self.set_state(GSMState::Attaching)?;

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
            self.set_state(GSMState::Deattached)?;
            return Err(Error::Network);
        }
        self.set_state(GSMState::Attached)?;

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
        self.set_state(GSMState::Deattached)?;

        Ok(())
    }
}
