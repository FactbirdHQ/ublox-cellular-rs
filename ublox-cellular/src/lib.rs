#![cfg_attr(not(test), no_std)]

//! # U-blox cellular
//!
//! This crate supports various U-blox cellular modules that are using AT commands based interface.
//! It can be used both on `no_std` and `std` platforms.
//!
//! ## Example
//!
//! By default following features are enabled: `toby-r2`, `socket-udp`, `socket-tcp`.
//!
//! An example to use a different modem with only enabling TCP support:
//!
//! ```toml
//! ublox-cellular-rs = { version = "0.4", default-features = false, features = ["sara-g3", "socket-tcp"] }
//! ```
//!
//! ### Clock trait
//!
//! To use this crate one must implement [`Clock`][clock] trait for a timer.
//! Notice that `Clock` uses [`Duration`][duration] and [`Instant`][instant] from [fugit] crate.
//!
//! Here is an example how it would look like for a `std` platform:
//!
//! ```
//! use ublox_cellular::fugit;
//! use ublox_cellular::prelude::*;
//!
//! pub struct SysTimer<const TIMER_HZ: u32> {
//!     start: std::time::Instant,
//!     duration: fugit::TimerDurationU32<TIMER_HZ>,
//! }
//!
//! impl<const TIMER_HZ: u32> SysTimer<TIMER_HZ> {
//!     pub fn new() -> Self {
//!         Self {
//!             start: std::time::Instant::now(),
//!             duration: fugit::TimerDurationU32::millis(0),
//!         }
//!     }
//! }
//!
//! impl<const TIMER_HZ: u32> Clock<TIMER_HZ> for SysTimer<TIMER_HZ> {
//!     type Error = std::convert::Infallible;
//!
//!     fn now(&mut self) -> fugit::TimerInstantU32<TIMER_HZ> {
//!         let millis = self.start.elapsed().as_millis();
//!         fugit::TimerInstantU32::from_ticks(millis as u32)
//!     }
//!
//!     fn start(&mut self, duration: fugit::TimerDurationU32<TIMER_HZ>) -> Result<(), Self::Error> {
//!         self.start = std::time::Instant::now();
//!         self.duration = duration.convert();
//!         Ok(())
//!     }
//!
//!     fn wait(&mut self) -> nb::Result<(), Self::Error> {
//!         if std::time::Instant::now() - self.start
//!             > std::time::Duration::from_millis(self.duration.ticks() as u64)
//!         {
//!             Ok(())
//!         } else {
//!             Err(nb::Error::WouldBlock)
//!         }
//!     }
//! }
//! ```
//!
//! ### Driver usage
//!
//! Modem driver usage examples can be found [here](https://github.com/BlackbirdHQ/ublox-cellular-rs/tree/master/examples).
//!
//! [clock]: prelude/trait.Clock.html
//! [duration]: ../fugit/duration/struct.Duration.html
//! [instant]: ../fugit/instant/struct.Instant.html
//!

mod client;
pub mod command;
mod config;
pub mod error;
mod network;
mod power;
mod registration;
mod services;

pub use atat::serde_bytes;
pub use ublox_sockets as sockets;

#[cfg(test)]
mod test_helpers;

pub use client::Device as GsmClient;
pub use config::{Config, NoPin};
pub use network::{ContextId, ProfileId};
pub use services::data::apn::{APNInfo, Apn};
pub use services::data::ssl::SecurityProfileId;
pub use services::data::DataService;

// Re-export atat and fugit
pub use atat;
pub use fugit;

/// Prelude - Include traits
pub mod prelude {
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use super::services::data::ssl::SSL;
    pub use atat::Clock;
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use embedded_nal::{
        IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpClientStack, UdpClientStack,
    };
}
