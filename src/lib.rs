#![no_std]

extern crate heapless;

extern crate at_rs as at;
#[macro_use]
extern crate nb;
extern crate no_std_net;

pub type ATClient<T> = at::client::ATClient<
    T,
    command::RequestType,
    heapless::consts::U5,
    heapless::consts::U5,
>;

#[cfg(test)]
#[macro_use]
mod test_helpers;

mod client;
pub mod soc;
pub mod gprs;

mod traits;

pub mod command;
pub mod error;
pub mod prelude;
pub mod socket;

pub use client::GSMClient;
