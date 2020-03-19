#![no_std]

mod client;
mod modules;
pub mod command;
pub mod error;
pub mod socket;

pub use modules::{gprs, gsm, soc, ssl};
pub use client::{GSMClient, Config as GSMConfig};

/// Prelude - Include traits
pub mod prelude {
    pub use super::gprs::GPRS;
    pub use super::gsm::GSM;
    pub use super::ssl::SSL;
    pub use embedded_nal::{TcpStack, UdpStack};
}
