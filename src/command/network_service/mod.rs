//! ### 7 - Network service

mod impl_;
pub mod responses;
pub mod types;

use atat::{atat_derive::ATATCmd, ATATCmd, Error};
use heapless::{consts, String};
use responses::*;
use types::*;

use super::NoResponse;

/// 7.14 Network registration status +CREG
///
/// Configures the network registration URC related to CS domain. Depending on the <n> parameter value, a URC
/// can be issued:
/// • +CREG: <stat> if <n>=1 and there is a change in the MT's circuit switched mode network registration status
/// in GERAN/UTRAN/E-UTRAN
/// • +CREG: <stat>[,<lac>,<ci>[,<AcTStatus>]] if <n>=2 and there is a change of the network cell in GERAN/
/// UTRAN/E-UTRAN
/// The parameters <AcTStatus>, <lac>, <ci> are provided only if available.
/// The read command provides the same information issued by the URC together with the current value of the
/// <n> parameter. The location information elements <lac>, <ci> and <AcTStatus>, if available, are returned only
/// when <n>=2 and the MT is registered with the network.
#[derive(Clone, ATATCmd)]
#[at_cmd("+CREG?", NetworkRegistrationStatus)]
pub struct SetNetworkRegistrationStatus {
    #[at_arg(position = 0)]
    n: NetworkRegistrationUrc,
}

#[derive(Clone, ATATCmd)]
#[at_cmd("+CREG?", NetworkRegistrationStatus)]
pub struct GetNetworkRegistrationStatus;
