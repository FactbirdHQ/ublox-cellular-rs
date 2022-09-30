//! Argument and parameter types used by Short Messages Service Commands and Responses
use atat::atat_derive::AtatEnum;

/// Indicates the basic message indication type
#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
pub enum MessageIndicationType {
    /// • 1: Voice Message Waiting (third level method) or Voice Message Waiting on Line 1
    /// (CPHS method)
    VoiceMessage = 1,
    /// • 2: Fax Message Waiting
    FaxMessage = 2,
    /// • 3: Electronic Mail Message Waiting
    EmailMessage = 3,
    /// • 4: Extended Message Type Waiting (i.e. see the 3GPP TS 23.038)
    ExtendedMessage = 4,
    /// • 5: Video Message Waiting
    VideoMessage = 5,
    /// • 6: Voice Message Waiting on Line 2 (CPHS method)
    VoiceMessageLine2 = 6,
    /// • 7: reserved for future use
    Reserved = 7,
}

/// Indicates whether the +UMWI URC is enabled or not
#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
pub enum MessageWaitingMode {
    /// • 0: disable the +UMWI URC
    Disabled = 0,
    /// • 1 (factory-programmed value): enable the +UMWI URC
    #[at_arg(default)]
    Enabled = 1,
}
