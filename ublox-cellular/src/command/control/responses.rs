use atat::atat_derive::AtatResp;

/// 15.9 UART data rate configuration +IPR
///
/// Returns the data rate at which the DCE accepts commands on the UART
/// interface.
#[derive(Clone, Debug, AtatResp)]
pub struct DataRate {
    #[at_arg(position = 0)]
    pub data_rate: u32,
}
