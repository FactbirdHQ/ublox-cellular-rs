#![cfg_attr(not(test), no_std)]
#![cfg_attr(feature = "async", allow(incomplete_features))]
// #![cfg_attr(feature = "async", feature(generic_const_exprs))]
// #![cfg_attr(feature = "async", feature(async_fn_in_trait))]
// #![cfg_attr(feature = "async", feature(type_alias_impl_trait))]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod command;
pub mod config;
pub mod error;
mod module_timing;

#[cfg(feature = "async")]
pub mod asynch;
