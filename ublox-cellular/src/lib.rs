#![cfg_attr(not(test), no_std)]

mod client;
pub mod command;
mod config;
pub mod error;
mod network;
mod power;
mod registration;
mod services;

pub use atat::serde_bytes;

#[cfg(test)]
mod test_helpers;

pub use client::Device as GsmClient;
pub use config::{Config, NoPin};
pub use network::{ContextId, ProfileId};
pub use services::data::apn::{APNInfo, Apn};
pub use services::data::ssl::SecurityProfileId;
pub use services::data::DataService;

// Re-export atat version in use
pub use atat;

pub type Instant<const TIMER_HZ: u32> = fugit::TimerInstantU32<TIMER_HZ>;

#[derive(Debug, PartialEq)]
pub enum ClockError {
    Infallible,
}

/// `Clock` provides all timing capabilities that are needed for the library.
///
/// Notice that `Clock` trait uses [fugit](https://lib.rs/crates/fugit) crate for `Duration` and `Instant`.
pub trait Clock<const TIMER_HZ: u32> {
    fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ>;

    fn start<T>(&mut self, count: T) -> Result<(), ClockError>
    where
        T: Into<fugit::MillisDurationU32>;

    fn wait(&mut self) -> Result<(), ClockError>;
}

/// Prelude - Include traits
pub mod prelude {
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use super::services::data::ssl::SSL;
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use embedded_nal::{TcpClientStack, UdpClientStack};
}
