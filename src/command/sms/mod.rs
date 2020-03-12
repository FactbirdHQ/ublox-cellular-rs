//! ### 11 - Short Messages Service

pub mod responses;
pub mod types;
pub mod urc;

use atat::{atat_derive::ATATCmd, ATATCmd, Error};
use heapless::{consts, String, Vec};
use responses::*;
use types::*;

use super::NoResponse;

#[derive(Clone, ATATCmd)]
#[at_cmd("", NoResponse, timeout_ms = 10000)]
pub struct AT;
