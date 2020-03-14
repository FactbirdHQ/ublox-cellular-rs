//! ### 11 - Short Messages Service

pub mod responses;
pub mod types;
pub mod urc;

use atat::{atat_derive::AtatCmd, AtatCmd, Error};
use heapless::{consts, String, Vec};
use responses::*;
use types::*;

use super::NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, timeout_ms = 10000)]
pub struct AT;
