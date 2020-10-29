use crate::{
    command::psn::{self, types::*},
    error::Error,
    GsmClient,
};
use embedded_hal::{blocking::delay::DelayMs, digital::{OutputPin, InputPin}};
use heapless::{consts, ArrayLength, Bucket, Pos, PowerOfTwo, String};

#[derive(Debug, Clone)]
pub enum Apn {
    Given(String<consts::U99>),
    Automatic,
}

impl Default for Apn {
    fn default() -> Self {
        Apn::Automatic
    }
}

#[derive(Debug, Clone, Default)]
pub struct APNInfo {
    pub apn: Apn,
    pub user_name: Option<String<consts::U64>>,
    pub password: Option<String<consts::U64>>,
}

impl APNInfo {
    pub fn new(apn: &str) -> Self {
        APNInfo {
            apn: Apn::Given(String::from(apn)),
            user_name: None,
            password: None,
        }
    }
}

pub trait GPRS {
    fn attach_gprs(&self) -> Result<(), Error>;
    fn check_gprs_attachment(&self) -> Result<bool, Error>;
    fn detach_gprs(&self) -> Result<(), Error>;
}

impl<C, DLY, N, L, RST, DTR, PWR, VINT> GPRS for GsmClient<C, DLY, N, L, RST, DTR, PWR, VINT>
where
    C: atat::AtatClient,
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
    fn check_gprs_attachment(&self) -> Result<bool, Error> {
        let psn::responses::PacketSwitchedNetworkData { param_tag, .. } =
            self.send_at(&psn::GetPacketSwitchedNetworkData {
                profile_id: 0,
                param: PacketSwitchedNetworkDataParam::PsdProfileStatus,
            })?;

        Ok(param_tag == 1)
    }

    fn attach_gprs(&self) -> Result<(), Error> {
        // let PDPContextState { status } = self.send_at(&psn::GetPDPContextState)?;
        // if status == PDPContextStatus::Deactivated {
        //     self.send_at(&psn::SetPDPContextState {
        //         status: PDPContextStatus::Deactivated
        //     })?;
        //     self.send_at(&psn::SetPDPContextState {
        //         status: PDPContextStatus::Activated
        //     })?;
        // }

        self.nwk_registration()?;
        self.try_connect(&self.config.apn_info)?;
        // let psn::responses::GPRSAttached { state } = self.send_at(&psn::GetGPRSAttached)?;

        // if state == GPRSAttachedState::Detached {
        //     // Attach GPRS
        //     self.send_at(&psn::SetGPRSAttached {
        //         state: GPRSAttachedState::Attached,
        //     })?;
        // }

        // if !self.check_gprs_attachment()? {
        //     // Set APN info
        //     let apn = match self.config.apn_info.apn {
        //         Apn::Given(ref apn) => apn.clone(),
        //         Apn::Automatic => unimplemented!(),
        //     };
        //     self.send_at(&psn::SetPacketSwitchedConfig {
        //         profile_id: 0,
        //         param: PacketSwitchedParam::APN(apn),
        //     })?;

        //     // Set auth mode
        //     // self.send_at(&psn::SetPacketSwitchedConfig {
        //     //     profile_id: 0,
        //     //     param: PacketSwitchedParam::Authentication(AuthenticationType::None),
        //     // })?;

        //     // // Set username
        //     // if let Some(ref user_name) = self.config.apn_info.user_name {
        //     //     self.send_at(&psn::SetPacketSwitchedConfig {
        //     //         profile_id: 0,
        //     //         param: PacketSwitchedParam::Username(user_name.clone()),
        //     //     })?;
        //     // }

        //     // // Set password
        //     // if let Some(ref password) = self.config.apn_info.password {
        //     //     self.send_at(&psn::SetPacketSwitchedConfig {
        //     //         profile_id: 0,
        //     //         param: PacketSwitchedParam::Password(password.clone()),
        //     //     })?;
        //     // }

        //     // // Set dynamic IP
        //     // self.send_at(&psn::SetPacketSwitchedConfig {
        //     //     profile_id: 0,
        //     //     param: PacketSwitchedParam::IPAddress(Ipv4Addr::unspecified().into()),
        //     // })?;

        //     // Activate IP
        //     self.send_at(&psn::SetPacketSwitchedAction {
        //         profile_id: 0,
        //         action: PacketSwitchedAction::Activate,
        //     })?;

        //     // Check profile status
        //     if !self.check_gprs_attachment()? {
        //         return Err(Error::Network);
        //     }
        // }

        Ok(())
    }

    fn detach_gprs(&self) -> Result<(), Error> {
        defmt::info!(
            "Detaching from network, {:?}",
            self.send_at(&psn::GetGPRSAttached)?.state
        );
        // Detach from network
        self.send_at(&psn::SetGPRSAttached {
            state: GPRSAttachedState::Detached,
        })?;
        defmt::info!("Detached!");

        Ok(())
    }
}
