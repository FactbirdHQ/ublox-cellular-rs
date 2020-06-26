use crate::{
    command::psn::{self, types::*},
    error::Error,
    GsmClient, State,
};
use embedded_hal::digital::v2::OutputPin;
use heapless::{consts, ArrayLength, String};
use no_std_net::Ipv4Addr;

#[derive(Debug, Clone, Default)]
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
    fn attach_gprs(&self) -> Result<(), Error>;
    fn detach_gprs(&self) -> Result<(), Error>;
}

impl<C, RST, DTR, N, L> GPRS for GsmClient<C, RST, DTR, N, L>
where
    C: atat::AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    fn attach_gprs(&self) -> Result<(), Error> {
        // match self.state.get() {
        //     State::Registered | State::Registering => return Err(Error::_Unknown),
        //     State::Attaching | State::Attached => return Ok(()),
        //     _ => {}
        // };

        self.state.set(State::Attaching);

        // Attach GPRS
        self.send_at(&psn::SetGPRSAttached {
            state: GPRSAttachedState::Attached,
        })?;

        let psn::responses::PacketSwitchedNetworkData { param_tag, .. } =
            self.send_at(&psn::GetPacketSwitchedNetworkData {
                profile_id: 0,
                param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
            })?;

        if param_tag == 0 {
            // Set APN info
            self.send_at(&psn::SetPacketSwitchedConfig {
                profile_id: 0,
                param: PacketSwitchedParam::APN(self.config.apn_info.apn.clone()),
            })?;

            // Set auth mode
            self.send_at(&psn::SetPacketSwitchedConfig {
                profile_id: 0,
                param: PacketSwitchedParam::Authentication(AuthenticationType::Auto),
            })?;

            // Set username
            if let Some(ref user_name) = self.config.apn_info.user_name {
                self.send_at(&psn::SetPacketSwitchedConfig {
                    profile_id: 0,
                    param: PacketSwitchedParam::Username(user_name.clone()),
                })?;
            }

            // Set password
            if let Some(ref password) = self.config.apn_info.password {
                self.send_at(&psn::SetPacketSwitchedConfig {
                    profile_id: 0,
                    param: PacketSwitchedParam::Password(password.clone()),
                })?;
            }

            // Set dynamic IP
            self.send_at(&psn::SetPacketSwitchedConfig {
                profile_id: 0,
                param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
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
                self.state.set(State::Detached);
                return Err(Error::Network);
            }
        }

        self.state.set(State::Attached);

        Ok(())
    }

    fn detach_gprs(&self) -> Result<(), Error> {
        #[cfg(features = "logging")]
        log::info!(
            "Detaching from network, {:?}",
            self.send_at(&psn::GetGPRSAttached)?.state
        );
        // Detach from network
        self.send_at(&psn::SetGPRSAttached {
            state: GPRSAttachedState::Detatched,
        })?;
        #[cfg(features = "logging")]
        log::info!("Detached!");
        self.state.set(State::Detached);

        Ok(())
    }
}
