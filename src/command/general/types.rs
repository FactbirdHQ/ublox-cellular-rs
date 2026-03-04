//! Argument and parameter types used by General Commands and Responses

use core::fmt::Write as _;

use atat::atat_derive::AtatEnum;
use serde::{Deserialize, Deserializer, Serialize};
#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum Snt {
    /// (default value): International Mobile station Equipment Identity (IMEI)
    IMEI = 0,
    /// International Mobile station Equipment Identity and Software Version number(IMEISV)
    IMEISV = 2,
    /// Software Version Number (SVN)
    SVN = 3,
    /// IMEI (not including the spare digit), the check digit and the SVN
    IMEIExtended = 255,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareVersion {
    major: u8,
    minor: u8,
}

impl FirmwareVersion {
    pub fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }
}

impl PartialOrd for FirmwareVersion {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.major.partial_cmp(&other.major) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.minor.partial_cmp(&other.minor)
    }
}

pub struct DeserializeError;

impl core::fmt::Display for DeserializeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Failed to deserialize version")
    }
}

impl core::str::FromStr for FirmwareVersion {
    type Err = DeserializeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.splitn(2, '.');
        let major = iter
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(DeserializeError)?;
        let minor = iter
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(DeserializeError)?;

        Ok(Self { major, minor })
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for FirmwareVersion {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}.{}", self.major, self.minor)
    }
}

impl Serialize for FirmwareVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut str = heapless::String::<7>::new();
        str.write_fmt(format_args!("{}.{}", self.major, self.minor))
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&str)
    }
}

impl<'de> Deserialize<'de> for FirmwareVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = atat::heapless_bytes::Bytes::<7>::deserialize(deserializer)?;
        core::str::FromStr::from_str(
            &core::str::from_utf8(s.as_slice()).map_err(serde::de::Error::custom)?,
        )
        .map_err(serde::de::Error::custom)
    }
}
