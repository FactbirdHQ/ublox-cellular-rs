use ufmt::derive::uDebug;
use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
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
