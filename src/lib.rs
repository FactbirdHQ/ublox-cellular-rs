#![no_std]

mod client;
pub mod command;
pub mod error;
mod hex;
mod modules;
pub mod socket;

pub use client::{Config as GSMConfig, GSMClient, State as GSMState};
pub use modules::{gprs, gsm, soc, ssl};

/// Prelude - Include traits
pub mod prelude {
    pub use super::gprs::GPRS;
    pub use super::gsm::GSM;
    pub use super::ssl::SSL;
    pub use embedded_nal::{TcpStack, UdpStack};
}
