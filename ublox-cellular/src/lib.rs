#![no_std]

mod client;
pub mod command;
pub mod error;
mod hex;
mod modules;
// mod state;
mod module_cfg;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
mod socket;

pub use client::{Config, GsmClient, State};
pub use modules::{gprs, gsm};

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub use modules::ssl;

#[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
pub mod sockets {
    pub use crate::modules::socket::*;
    pub use crate::socket::*;
}

pub use atat;

/// Prelude - Include traits
pub mod prelude {
    pub use super::gprs::GPRS;
    pub use super::gsm::GSM;

    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use super::ssl::SSL;
    #[cfg(any(feature = "socket-udp", feature = "socket-tcp"))]
    pub use embedded_nal::{TcpStack, UdpStack};
}
