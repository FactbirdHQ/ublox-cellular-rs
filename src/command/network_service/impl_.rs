use super::types::NetworkRegistrationStat;
use crate::network::Error;

impl NetworkRegistrationStat {
    #[must_use]
    pub fn is_access_alive(&self) -> bool {
        matches!(self, Self::Registered | Self::RegisteredRoaming)
    }

    pub fn registration_ok(self) -> Result<Self, Error> {
        match self {
            Self::RegistrationDenied => Err(Error::RegistrationDenied),
            _ => Ok(self),
        }
    }
}
