//! Responses for General Commands
use atat::atat_derive::AtatResp;
use heapless::consts;
use serde_at::CharVec;

/// 4.1 Manufacturer identification
/// Text string identifying the manufacturer.
#[derive(Clone, Debug, AtatResp)]
pub struct ManufacturerId {
    #[at_arg(position = 0)]
    pub manufacturer: CharVec<consts::U10>,
}

/// 4.3 Model identification
/// Text string identifying the model identification.
#[derive(Clone, Debug, AtatResp)]
pub struct ModelId {
    #[at_arg(position = 0)]
    pub model: CharVec<consts::U16>,
}

/// 4.5 Firmware version identification
/// Returns the firmware version of the module.
#[derive(Clone, Debug, AtatResp)]
pub struct FirmwareVersion {
    #[at_arg(position = 0)]
    pub version: CharVec<consts::U10>,
}

/// 4.7 IMEI identification +CGSN
///
/// Returns the product serial number, the International Mobile Equipment
/// Identity (IMEI) of the MT.
#[derive(Clone, Debug, AtatResp)]
pub struct IMEI {
    #[at_arg(position = 0)]
    pub imei: u64,
}

/// 4.9 Identification information I
///
/// Returns some module information as the module type number and some details
/// about the firmware version.
#[derive(Clone, Debug, AtatResp)]
pub struct IdentificationInformationResponse {
    pub app_ver: CharVec<consts::U32>,
}

/// 4.11 International mobile subscriber identification +CIM
///
/// Request the IMSI (International Mobile Subscriber Identity).
#[derive(Clone, Debug, AtatResp)]
pub struct CIMI {
    /// International Mobile Subscriber Identity
    #[at_arg(position = 0)]
    pub imsi: u64,
}

/// 4.12 Card identification +CCID
///
/// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a
/// serial number identifying the SIM.
#[derive(Clone, Debug, AtatResp)]
pub struct CCID {
    #[at_arg(position = 0)]
    pub ccid: u128,
}
