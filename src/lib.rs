#![no_std]
#![allow(unused)]
extern crate heapless;

extern crate atat;
extern crate nb;
extern crate no_std_net;
extern crate ufmt;

#[cfg(test)]
#[macro_use]
mod test_helpers;

#[cfg(test)]
#[macro_use]
extern crate std;

mod client;
pub mod soc;
pub mod gprs;

pub mod command;
pub mod error;
pub mod prelude;
pub mod socket;

pub use client::GSMClient;
