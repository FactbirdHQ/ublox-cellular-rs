//! ### 4 - General Commands
pub mod responses;
pub mod types;

use atat::atat_derive::AtatCmd;
use responses::{CCID, CIMI, FirmwareVersion, IMEI, IdentificationInformationResponse, ManufacturerId, ModelId};
use types::Snt;

/// 4.1 Manufacturer identification +CGMI
///
/// Text string identifying the manufacturer.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMI", ManufacturerId)]
pub struct GetManufacturerId;

/// 4.3 Model identification +CGMM
///
/// Text string identifying the model identification.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMM", ModelId)]
pub struct GetModelId;

/// 4.5 Firmware version identification +CGMR
///
/// Returns the firmware version of the module.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGMR", FirmwareVersion)]
pub struct GetFirmwareVersion;

/// 4.7 IMEI identification +CGSN
///
/// Returns the product serial number, the International Mobile Equipment Identity (IMEI) of the MT.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CGSN", IMEI)]
pub struct GetIMEI {
    #[at_arg(position = 0)]
    pub snt: Option<Snt>,
}

/// 4.9 Identification information I
///
/// Returns some module information as the module type number and some details
/// about the firmware version.
///
/// **Notes:**
/// - The information text response of ATI9 contains the modem version and the
///   application version of the module where applicable; it returns "Undefined"
///   where not applicable.
#[derive(Clone, AtatCmd)]
#[at_cmd("I", IdentificationInformationResponse, value_sep = false)]
pub struct IdentificationInformation {
    #[at_arg(position = 0)]
    pub n: u8,
}

/// 4.11 International mobile subscriber identification +CIMI
///
/// Request the IMSI (International Mobile Subscriber Identity)
#[derive(Clone, AtatCmd)]
#[at_cmd("+CIMI", CIMI)]
pub struct GetCIMI;

/// 4.12 Card identification +CCID
///
/// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CCID", CCID)]
pub struct GetCCID;
