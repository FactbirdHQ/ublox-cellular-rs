//! ### 3 - IPC - Inter Processor Communication
use atat::atat_derive::AtatCmd;

use super::NoResponse;

/// 3.1 Multiplexing mode +CMUX
///
/// Enables the multiplexing protocol control channel as defined in 3GPP TS
/// 27.010 [104]. The command sets the parameters for the control channel. The
/// result code is returned using the old interface speed. The parameters become
/// active only after sending the OK result code. The usage of +CMUX set command
/// during the multiplexing is not allowed.
#[derive(Clone, AtatCmd)]
#[at_cmd("+CMUX", NoResponse)]
pub struct SetMultiplexing {
    /// Multiplexer transparency mechanism:
    #[at_arg(position = 0)]
    pub mode: u8,

    /// The way in which the multiplexer control channel is set up:
    #[at_arg(position = 1)]
    pub subset: Option<u8>,

    /// Transmission rate. The allowed range is 0-7.
    /// 0, 9600, 19200, 38400, 57600, 115200, 230400, 460800
    #[at_arg(position = 2)]
    pub port_speed: Option<u8>,

    /// Maximum frame size
    ///
    /// - Allowed range is 1-1509.
    /// - The default value is 31.
    #[at_arg(position = 3)]
    pub n1: Option<u16>,

    /// Acknowledgement timer in units of ten milliseconds.
    ///
    /// - The allowed range is 1-255
    #[at_arg(position = 4)]
    pub t1: Option<u8>,

    /// Maximum number of re-transmissions
    #[at_arg(position = 5)]
    pub n2: Option<u8>,

    /// Response timer for the multiplexer control channel in units of ten
    /// milliseconds.
    #[at_arg(position = 6)]
    pub t2: Option<u8>,

    /// Wake up response timer.
    #[at_arg(position = 7)]
    pub t3: Option<u8>,

    /// Window size, for advanced operation with Error Recovery options.
    #[at_arg(position = 8)]
    pub k: Option<u8>,
}
