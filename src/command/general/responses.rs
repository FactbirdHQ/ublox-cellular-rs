//! 4 Responses for General Commands
use heapless::{consts, String};
use serde::Deserialize;
pub struct NoResponse;


/// 4.1 Manufacturer identification
/// Text string identifying the manufacturer.
// #[derive(Deserialize)]
pub struct ManufacturerId {
    // #[atat_(position = 0)]
    pub id: String<consts::U64>,
}

/// 4.7 IMEI identification +CGSN
/// Returns the product serial number, the International Mobile Equipment Identity (IMEI) of the MT.
// #[derive(Deserialize)]
pub struct IMEI {
    // #[atat_(position = 0)]
    pub imei: u64
}

/// 4.12 Card identification +CCID
/// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
// #[derive(Deserialize)]
pub struct CCID{
    // #[atat_(position = 0)]
    pub ccid : u64
}