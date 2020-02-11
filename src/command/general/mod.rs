//! 4 General Commands
pub mod responses;
pub mod types;
use at::{Error, ATATCmd};
use heapless::{consts, String};
use responses::*;
use types::*;
use at::atat_derive::ATATCmd;
use serde::Serialize;


/// 4.1 Manufacturer identification +CGMI
/// Text string identifying the manufacturer.
// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CGMI", ManufacturerId)]
pub struct GetManufacturerId;



/// 4.7 IMEI identification +CGSN
/// Returns the product serial number, the International Mobile Equipment Identity (IMEI) of the MT.
// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CGSN", IMEI)]
pub struct GetIMEI {
    #[atat_(position = 0)]
    snt: Option<Snt>,
}


/// 4.12 Card identification +CCID
/// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
// #[derive(Clone, ATATCmd)]
// #[at_cmd("+CCID" CCID)]
pub struct GetCCID;



// #[derive(Clone, ATATCmd)]
// #[atat("+USORD", SocketData, timeout_ms = 10000, abortable = true)]
pub struct ReadSocketData {
    #[atat_(position = 0)]
    socket: SocketHandle,
    #[atat_(position = 1)]
    length: usize,
}