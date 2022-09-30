//! Argument and parameter types used by Network service Commands and Responses
use atat::atat_derive::AtatEnum;
use heapless::String;

/// Is used to chose whether the network selection is automatically done by the
/// MT or is forced by this command to the operator <oper> given in the format
/// <format>
#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OperatorSelectionMode {
    /// • 0 (default value and factory-programmed value): automatic (<oper> field is ignored)
    Automatic = 0,
    /// • 1: manual
    Manual = 1,
    /// • 2: deregister from network
    Deregister = 2,
    /// • 3: set only <format>
    FormatOnly = 3,
    /// • 4: manual/automatic
    ManualAutomatic = 4,
    /// • 5: extended network search
    ExtendedNetworkSearch = 5,
    /// • 6: extended network search without the tags (e.g. MCC, RxLev will not be printed,
    /// see the syntax and the command example)
    ExtendedNetworkSearchWithoutTags = 6,
    /// • 8: network timing advance search
    NetworkTimingAdvanceSearch = 8,

    #[at_arg(default)]
    Unknown,
}
/// Indicates the radio access technology
#[derive(Debug, Clone, Copy, PartialEq, Eq, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RatAct {
    /// • 0: GSM
    Gsm = 0,
    /// • 1: GSM COMPACT
    GsmCompact = 1,
    /// • 2: UTRAN
    Utran = 2,
    /// • 3: GSM/GPRS with EDGE availability
    GsmGprsEdge = 3,
    /// • 4: UTRAN with HSDPA availability
    UtranHspda = 4,
    /// • 5: UTRAN with HSUPA availability
    UtranHsupa = 5,
    /// • 6: UTRAN with HSDPA and HSUPA availability
    UtranHspdaHsupa = 6,
    /// • 7: LTE
    Lte = 7,
    /// • 8: EC-GSM-IoT (A/Gb mode)
    EcGsmIot = 8,
    /// • 9: E-UTRAN (NB-S1 mode)
    Eutran = 9,
    #[at_arg(default)]
    Unknown = 10,
}

#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum NetworkRegistrationUrcConfig {
    /// • 0 (default value and factory-programmed value): network registration URC disabled
    UrcDisabled = 0,
    /// • 1: network registration URC +CREG: <stat> enabled
    UrcEnabled = 1,
    /// • 2: network registration and location information URC +CREG: <stat>[,<lac>,<ci>[,
    /// <AcTStatus>]] enabled
    UrcVerbose = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NetworkRegistrationStat {
    /// • 0: not registered, the MT is not currently searching a new operator to register to
    NotRegistered = 0,
    /// • 1: registered, home network
    Registered = 1,
    /// • 2: not registered, but the MT is currently searching a new operator to register to
    NotRegisteredSearching = 2,
    /// • 3: registration denied
    RegistrationDenied = 3,
    /// • 4: unknown (e.g. out of GERAN/UTRAN/E-UTRAN coverage)
    Unknown = 4,
    /// • 5: registered, roaming
    RegisteredRoaming = 5,
    /// • 6: registered for "SMS only", home network (applicable only when <AcTStatus>
    /// indicates E-UTRAN)
    RegisteredSmsOnly = 6,
    /// • 7: registered for "SMS only", roaming (applicable only when <AcTStatus> indicates
    /// E-UTRAN)
    RegisteredSmsOnlyRoaming = 7,
    /// • 9: registered for "CSFB not preferred", home network (applicable only when
    /// <AcTStatus> indicates E-UTRAN)
    RegisteredCsfbNotPerferred = 9,
    /// • 10: registered for "CSFB not preferred", roaming (applicable only when <AcTStatus>
    /// indicates E-UTRAN)
    RegisteredCsfbNotPerferredRoaming = 10,
}

/// Indicates the preferred access technology
#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RatPreferred {
    /// • 0: GSM / GPRS / eGPRS
    GsmGprsEgprs = 0,
    /// • 2: UTRAN
    Utran = 2,
    /// • 3: LTE
    Lte = 3,
}

/// Indicates the radio access technology
#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RadioAccessTechnologySelected {
    /// • 0: GSM / GPRS / eGPRS (single mode)
    #[at_arg(value = 0)]
    GsmGprsEGprs,
    /// • 1: GSM / UMTS (dual mode)
    #[at_arg(value = 1)]
    GsmUmts(RatPreferred),
    /// • 2: UMTS (single mode)
    #[at_arg(value = 2)]
    Umts,
    /// • 3: LTE (single mode)
    #[at_arg(value = 3)]
    Lte,
    /// • 4: GSM / UMTS / LTE (tri mode)
    #[at_arg(value = 4)]
    GsmUmtsLte(RatPreferred, RatPreferred),
    /// • 5: GSM / LTE (dual mode)
    #[at_arg(value = 5)]
    GsmLte(RatPreferred),
    /// • 6: UMTS / LTE (dual mode)
    #[at_arg(value = 6)]
    UmtsLte(RatPreferred),
}

#[derive(Debug, Clone, PartialEq, Eq, AtatEnum)]
pub enum OperatorNameFormat {
    #[at_arg(value = 0)]
    Long(String<24>),
    #[at_arg(value = 1)]
    Short(String<10>),
    #[at_arg(value = 2)]
    Numeric(String<6>),
}
