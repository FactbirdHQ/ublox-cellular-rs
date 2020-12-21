#![no_std]
mod client;
pub mod command;
mod config;
pub mod error;
mod module_cfg;
mod network;
mod services;
mod state;

pub use client::Device as GsmClient;
pub use config::Config;
pub use network::{ContextId, ProfileId};
pub use services::data::apn::{APNInfo, Apn};
pub use services::data::tls::SecurityProfileId;
pub use state::State;

// Re-export atat version in use
pub use atat;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub mod sockets {
    pub use super::services::data::socket::*;
}

/// Prelude - Include traits
pub mod prelude {
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use embedded_nal::{tls::TlsConnect, TcpClientStack, UdpClientStack};
}
