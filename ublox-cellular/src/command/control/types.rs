//! Argument and parameter types used by V24 control and V25ter Commands and Responses

use atat::atat_derive::AtatEnum;

#[derive(Clone, PartialEq, AtatEnum)]
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

#[derive(Clone, PartialEq, AtatEnum)]
pub enum SoftwareFlowControl {
    /// - 0: Software flow control off
    None = 0,
    /// - 1: DC1/DC3 on circuit 103 and 104 (XON/XOFF)
    Circuit103_104 = 1,
    /// - 3: (**default value**): DCE_by_DTE on circuit 105 (RTS) and DTE_by_DCE on circuit 106 (CTS)
    Circuit105_106 = 3,
}

#[derive(Clone, PartialEq, AtatEnum)]
#[at_enum(u32)]
pub enum BaudRate {
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    B0 = 0,
    #[cfg(any(feature = "lisa_u1", feature = "lisa_u2", feature = "sara_u2",))]
    B1200 = 1200,
    #[cfg(any(
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    B2400 = 2400,
    #[cfg(any(
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    B4800 = 4800,
    B9600 = 9600,
    B19200 = 19200,
    B38400 = 38400,
    B57600 = 57600,
    B115200 = 115200,

    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
    ))]
    B230400 = 230400,
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
    ))]
    B460800 = 460800,
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
    ))]
    B921600 = 921600,
    #[cfg(any(feature = "toby_r2", feature = "lara_r2",))]
    B3000000 = 3000000,
    #[cfg(any(feature = "toby_r2", feature = "lara_r2",))]
    B3250000 = 3250000,
    #[cfg(any(feature = "toby_r2", feature = "lara_r2",))]
    B6000000 = 6000000,
    #[cfg(any(feature = "toby_r2", feature = "lara_r2",))]
    B6500000 = 6500000,
}
