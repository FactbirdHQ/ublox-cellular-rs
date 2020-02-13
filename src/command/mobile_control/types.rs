use serde_repr::{Deserialize_repr, Serialize_repr};
use ufmt::derive::uDebug;

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Functionality {
    /// Sets the MT to minimum functionality (disable both transmit and receive RF
    /// circuits by deactivating both CS and PS services)
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    Minimum = 0,

    /// (factory-programmed value): sets the MT to full functionality, e.g. from airplane
    /// mode or minimum functionality
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    Full = 1,

    /// Disables both transmit and receive RF circuits by deactivating both CS and PS
    /// services and sets the MT into airplane mode. Airplane mode is persistent between
    /// power cycles triggered by +CFUN=16 or +CPWROFF (where supported)
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4"
    ))]
    AirplaneMode = 4,

    /// Enables the SIM toolkit interface in dedicated mode and fetching of proactive
    /// commands by SIM Application Toolkit from the SIM card
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    DedicatedMode = 6,

    /// Disables the SIM toolkit interface and fetching of proactive commands by
    /// SIM Application Toolkit from the SIM card
    #[cfg(any(
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    DisableSimToolkit = 7,
    #[cfg(any(
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    DisableSimToolkit_ = 8,

    /// Enables the SIM toolkit interface in raw mode and fetching of proactive
    /// commands by SIM Application Toolkit from the SIM card
    #[cfg(any(
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2"
    ))]
    RawMode = 9,

    /// MT silent reset (with detach from network and saving of NVM parameters),
    /// without reset of the SIM card
    #[cfg(any(
        feature = "toby_l2",
        feature = "mpci_l2",
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    SilentReset = 15,

    /// MT silent reset (with detach from network and saving of NVM parameters), with
    /// reset of the SIM card
    #[cfg(any(
        feature = "lisa_u1",
        feature = "lisa_u2",
        feature = "sara_u2",
        feature = "toby_r2",
        feature = "lara_r2",
        feature = "toby_l4",
        feature = "leon_g1",
        feature = "sara_g3",
        feature = "sara_g4"
    ))]
    SilentResetWithSimReset = 16,

    /// Sets the MT to minimum functionality by deactivating CS and PS services and
    /// the SIM card
    #[cfg(any(feature = "toby_l2", feature = "mpci_l2", feature = "toby_l4"))]
    MinimumWithoutSim = 19,

    /// Sets the MT in a deep low power state "HALT" (with detach from the network
    /// and saving of the NVM parameters); the only way to wake up the module is a power
    /// cycle or a module reset
    #[cfg(any(feature = "toby_l2", feature = "mpci_l2"))]
    Halt = 127,
}

/// Reset mode. This parameter can be used only when <fun> (ModuleFunctionality) is 1, 4 or 19
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ResetMode {
    /// Do not reset the MT before setting it to the selected <fun>
    DontReset = 0,
    /// Performs a MT silent reset (with detach from network and saving of nvm
    /// parameters) with reset of the SIM card before setting it to the selected <fun>
    Reset = 1,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TerminationErrorMode {
    /// +CME ERROR: <err> result code disabled and ERROR used
    Disabled = 0,
    /// +CME ERROR: <err> result code enabled and numeric <err> values used
    Enabled = 1,
    /// +CME ERROR: <err> result code enabled and verbose <err> values used
    Verbose = 2,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PowerMode {
    ///MT is switched on with minimum functionality
    Minimum = 0,
    ///MT is switched on
    On = 1,
    ///MT is in "airplane mode"
    AirplaneMode = 4,
    ///MT is in minimum functionality with SIM deactivated
    MinimumWithoutSim = 19,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum STKMode {
    ///the SIM-toolkit interface in dedicated mode and fetching of proactive commands by SIM-APPL from the SIM-card are enabled
    DedicatedMode = 6,
    /// the SIM-toolkit interface is disabled; fetching of proactive commands by SIM-APPL from the SIM-card is enabled
    Disabled = 0,
    ///the SIM-toolkit interface in raw mode and fetching of proactive commands by SIM-APPL from the SIM-card are enabled
    RawMode = 9,
}

#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ReportMobileTerminationErrorStatus {
    ///+CME ERROR: <err> result code disabled and ERROR used
    DisabledERRORused = 0,
    ///+CME ERROR: <err> result code enabled and numeric <err> values used
    EnabledCodeUsed = 1,
    ///+CME ERROR: <err> result code enabled and verbose <err> values used
    EnabledVerbose = 2,
}
