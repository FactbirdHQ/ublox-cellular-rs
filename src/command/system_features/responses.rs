//! 4 Responses for General Commands
use heapless::{consts, String};
use serde::Deserialize;



/// 19.8 Power saving control (Power SaVing) +UPSV
// #[derive(Deserialize)]
pub struct PowerSavingControl{
    // #[atat_(position = 0)]
    mode: PowerSavingMode,
    // #[atat_(position = 1)]
    timeout: Option<Seconds>,
}

