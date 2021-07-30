#![cfg_attr(not(test), no_std)]

mod client;
pub mod command;
mod config;
pub mod error;
mod network;
mod power;
mod registration;
mod services;

pub use serde_bytes;

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

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub mod sockets {
    pub use super::services::data::socket::*;
}

/// Prelude - Include traits
pub mod prelude {
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use super::services::data::ssl::SSL;
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use embedded_nal::{TcpClientStack, UdpClientStack};
}
