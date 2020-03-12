//! Argument and parameter types used by Network service Commands and Responses
use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum NetworkRegistrationUrc {
    /// • 0 (default value and factory-programmed value): network registration URC disabled
    UrcDisabled = 0,
    /// • 1: network registration URC +CREG: <stat> enabled
    UrcEnabled = 1,
    /// • 2: network registration and location information URC +CREG: <stat>[,<lac>,<ci>[,
    /// <AcTStatus>]] enabled
    UrcVerbose = 2,
}

#[derive(uDebug, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum NetworkRegistrationStat {
    /// • 0: not registered, the MT is not currently searching a new operator to register to
    NotRegistered = 0,
    /// • 1: registered, home network
    Registered = 1,
    /// • 2: not registered, but the MT is currently searching a new operator to register to
    NotRegisteredSearching = 2,
    /// • 3: registration denied
    RegistrationDenied = 3,
    /// • 4: unknown (e.g. out of GERAN/UTRAN/E-UTRAN coverage)
    Unknown = 4,
    /// • 5: registered, roaming
    RegisteredRoaming = 5,
    /// • 6: registered for "SMS only", home network (applicable only when <AcTStatus>
    /// indicates E-UTRAN)
    RegisteredSmsOnly = 6,
    /// • 7: registered for "SMS only", roaming (applicable only when <AcTStatus> indicates
    /// E-UTRAN)
    RegisteredSmsOnlyRoaming = 7,
    /// • 9: registered for "CSFB not preferred", home network (applicable only when
    /// <AcTStatus> indicates E-UTRAN)
    RegisteredCsfbNotPerferred = 9,
    /// • 10: registered for "CSFB not preferred", roaming (applicable only when <AcTStatus>
    /// indicates E-UTRAN)
    RegisteredCsfbNotPerferredRoaming = 10,
}
