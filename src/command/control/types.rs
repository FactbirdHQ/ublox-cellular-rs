//! Argument and parameter types used by V24 control and V25ter Commands and Responses

use atat::atat_derive::AtatEnum;

/// Indicates the behavior of circuit 109
#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum Circuit109Behaviour {
    /// 0: DCE always presents ON condition on circuit 109
    AlwaysPresent = 0,
    /// 1 (default value and factory-programmed value): circuit 109 changes in
    /// accordance with the Carrier detect status; ON if the Carrier is
    /// detected, OFF otherwise
    ChangesWithCarrier = 1,
}

#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum Echo {
    /// 0: Echo off
    Off = 0,
    /// 1 (default value and factory-programmed value): Echo on
    On = 1,
}

/// Indicates the behavior of circuit 108
#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum Circuit108Behaviour {
    /// 0: the DCE ignores circuit 108/2
    Ignore = 0,
    /// 1 (default value and factory-programmed value): upon an ON-to-OFF
    /// transition of circuit 108/2, the DCE enters online command state and
    /// issues the final result code
    OnlineCommandState = 1,
    /// 2: upon an ON-to-OFF transition of circuit 108/2, the DCE performs an
    /// orderly cleardown of the call. The automatic answer is disabled while
    /// circuit 108/2 remains OFF
    OrderlyCleardown = 2,
}

#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum FlowControl {
    /// - 0: disable DTE flow control
    Disabled = 0,
    /// - 3 (**default and factory-programmed value**): enable the RTS/CTS DTE flow
    ///   control
    RtsCts = 3,
    /// - 4: enable the XON/XOFF DTE flow control
    /// - 5: enable the XON/XOFF DTE flow control
    /// - 6: enable the XON/XOFF DTE flow control
    XonXoff = 4,
}

#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum SoftwareFlowControl {
    /// - 0: Software flow control off
    None = 0,
    /// - 1: DC1/DC3 on circuit 103 and 104 (XON/XOFF)
    Circuit103_104 = 1,
    /// - 3: (**default value**): DCE_by_DTE on circuit 105 (RTS) and DTE_by_DCE on circuit 106 (CTS)
    Circuit105_106 = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[at_enum(u32)]
pub enum BaudRate {
    #[cfg(any(
        feature = "toby-l2",
        feature = "mpci-l2",
        feature = "sara-u2",
        feature = "toby-r2",
        feature = "lara-r2",
        feature = "toby-l4",
        feature = "leon-g1",
        feature = "sara-g3",
        feature = "sara-g4"
    ))]
    B0 = 0,
    #[cfg(any(feature = "lisa-u1", feature = "lisa-u2", feature = "sara-u2",))]
    B1200 = 1200,
    #[cfg(any(
        feature = "lisa-u1",
        feature = "lisa-u2",
        feature = "sara-u2",
        feature = "leon-g1",
        feature = "sara-g3",
        feature = "sara-g4"
    ))]
    B2400 = 2400,
    #[cfg(any(
        feature = "lisa-u1",
        feature = "lisa-u2",
        feature = "sara-u2",
        feature = "leon-g1",
        feature = "sara-g3",
        feature = "sara-g4"
    ))]
    B4800 = 4800,
    B9600 = 9600,
    B19200 = 19200,
    B38400 = 38400,
    B57600 = 57600,
    B115200 = 115_200,

    #[cfg(any(
        feature = "toby-l2",
        feature = "mpci-l2",
        feature = "lisa-u1",
        feature = "lisa-u2",
        feature = "sara-u2",
        feature = "toby-r2",
        feature = "lara-r2",
        feature = "lara-r6",
        feature = "toby-l4",
    ))]
    B230400 = 230_400,
    #[cfg(any(
        feature = "toby-l2",
        feature = "mpci-l2",
        feature = "lisa-u1",
        feature = "lisa-u2",
        feature = "sara-u2",
        feature = "toby-r2",
        feature = "lara-r2",
        feature = "lara-r6",
        feature = "toby-l4",
    ))]
    B460800 = 460_800,
    #[cfg(any(
        feature = "toby-l2",
        feature = "mpci-l2",
        feature = "lisa-u1",
        feature = "lisa-u2",
        feature = "sara-u2",
        feature = "toby-r2",
        feature = "lara-r2",
        feature = "lara-r6",
        feature = "toby-l4",
    ))]
    B921600 = 921_600,
    #[cfg(any(feature = "toby-r2", feature = "lara-r2", feature = "lara-r6"))]
    B3000000 = 3_000_000,
    #[cfg(any(feature = "toby-r2", feature = "lara-r2",))]
    B3250000 = 3_250_000,
    #[cfg(any(feature = "toby-r2", feature = "lara-r2",))]
    B6000000 = 6_000_000,
    #[cfg(any(feature = "toby-r2", feature = "lara-r2",))]
    B6500000 = 6_500_000,
}
