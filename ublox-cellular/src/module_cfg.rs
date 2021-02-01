#[cfg(feature = "toby-r2")]
pub mod constants {
    pub const NRST_PULL_TIME_MS: u32 = 100;
    pub const PWR_ON_PULL_TIME_MS: u32 = 300;
    pub const BOOT_WAIT_TIME_MS: u32 = 10000;
}
