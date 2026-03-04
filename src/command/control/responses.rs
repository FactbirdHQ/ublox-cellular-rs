//! Responses for Control Commands
use super::types::BaudRate;
use atat::atat_derive::AtatResp;

#[derive(Clone, Debug, PartialEq, Eq, AtatResp)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DataRate {
    #[at_arg(position = 0)]
    pub rate: BaudRate,
}
