//! Argument and parameter types used by IPC - Inter Processor Communication

use atat::atat_derive::AtatEnum;
use serde::{Deserialize, Deserializer, Serialize};

use crate::command::control::types::BaudRate;

#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum MultiplexingBaudrate {
    Auto = 0,
    B9600,
    B19200,
    B38400,
    B57600,
    B115200,
    B230400,
    B460800,
}

impl From<BaudRate> for MultiplexingBaudrate {
    fn from(value: BaudRate) -> Self {
        match value {
            BaudRate::B9600 => Self::B9600,
            BaudRate::B19200 => Self::B19200,
            BaudRate::B38400 => Self::B38400,
            BaudRate::B57600 => Self::B57600,
            BaudRate::B115200 => Self::B115200,
            BaudRate::B230400 => Self::B230400,
            BaudRate::B460800 => Self::B460800,
            _ => Self::Auto,
        }
    }
}
