use super::types::*;
use crate::network::Error;

impl NetworkRegistrationStat {
    pub fn is_access_alive(&self) -> bool {
        matches!(
            self,
            NetworkRegistrationStat::Registered | NetworkRegistrationStat::RegisteredRoaming
        )
    }

    pub fn registration_ok(self) -> Result<Self, Error> {
        match self {
            NetworkRegistrationStat::RegistrationDenied => Err(Error::RegistrationDenied),
            _ => Ok(self),
        }
    }
}
