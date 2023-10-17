//! Responses for System features Commands
use super::types::{FSFactoryRestoreType, NVMFactoryRestoreType, PowerSavingMode, Seconds};
use atat::atat_derive::AtatResp;

/// 19.8 Power saving control (Power Saving) +UPSV
#[derive(AtatResp)]
pub struct PowerSavingControl {
    #[at_arg(position = 0)]
    pub mode: PowerSavingMode,
    #[at_arg(position = 1)]
    pub timeout: Option<Seconds>,
}

/// 19.25 Restore factory configuration +UFACTORY
#[derive(AtatResp)]
pub struct FactoryConfiguration {
    #[at_arg(position = 0)]
    pub fs_op: FSFactoryRestoreType,
    #[at_arg(position = 1)]
    pub nvm_op: NVMFactoryRestoreType,
}
