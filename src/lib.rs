#![cfg_attr(not(test), no_std)]
#![allow(async_fn_in_trait)]

#[cfg(all(feature = "ppp", feature = "internal-network-stack"))]
compile_error!("You may not enable both `ppp` and `internal-network-stack` features.");

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod command;
pub mod config;
pub mod error;
mod modules;
mod registration;

pub mod asynch;

use command::control::types::BaudRate;
pub const DEFAULT_BAUD_RATE: BaudRate = BaudRate::B115200;
