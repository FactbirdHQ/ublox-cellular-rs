use core::str::FromStr;

#[derive(Debug, PartialEq, defmt::Format)]
pub enum UbloxError {
    Generic,
    Cme(CmeError),
    Cms(CmsError),
}

impl From<atat::GenericError> for UbloxError {
    fn from(_: atat::GenericError) -> Self {
        Self::Generic
    }
}

impl FromStr for UbloxError {
    // This error will always get mapped to `atat::Error::Parse`
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(err) = s.trim().strip_prefix("+CME ERROR:") {
            Self::Cme(err.parse().unwrap_or(CmeError::Unknown))
        } else if let Some(err) = s.trim().strip_prefix("+CMS ERROR:") {
            Self::Cms(err.parse().unwrap_or(CmsError::Unknown))
        } else {
            Self::Generic
        })
    }
}

/// Message service error result codes +CMS ERROR
#[derive(Debug, PartialEq, defmt::Format)]
pub enum CmsError {
    Empty,
    Unknown,
}

impl FromStr for CmsError {
    // This error will always get mapped to `atat::Error::Parse`
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            _ => return Err(()),
        })
    }
}

// FIXME: Re-enable remaining error types once https://github.com/knurling-rs/defmt/pull/302 is released
/// Mobile termination error result codes +CME ERROR
#[derive(Debug, PartialEq, defmt::Format)]
pub enum CmeError {
    // #[at_arg(0, "Phone failure")]
    PhoneFailure,
    NoConnectionToPhone,
    PhoneAdaptorLinkReserved,
    OperationNotAllowed,
    OperationNotSupported,
    PhSimPinRequired,
    PhFsimPinRequired,
    PhFsimPukRequired,
    SimNotInserted,
    SimPinRequired,
    SimPukRequired,
    SimFailure,
    SimBusy,
    SimWrong,
    IncorrectPassword,
    SimPin2Required,
    SimPuk2Required,
    MemoryFull,
    InvalidIndex,
    NetworkNotFound,
    MemoryFailure,
    TextStringTooLong,
    InvalidCharactersInTextString,
    DialStringTooLong,
    InvalidCharactersInDialString,
    NoNetworkService,
    NetworkTimeout,
    NetworkNotAllowedEmergencyCallsOnly,
    NetworkPersonalisationPinRequired,
    NetworkPersonalisationPukRequired,
    NetworkSubsetPersonalisationPinRequired,
    NetworkSubsetPersonalisationPukRequired,
    ServiceProviderPersonalisationPinRequired,
    ServiceProviderPersonalisationPukRequired,
    CorporatePersonalisationPinRequired,
    CorporatePersonalisationPukRequired,
    IncorrectParameters,
    CommandImplementedButCurrentlyDisabled,
    CommandAbortedByUser,
    NotAttachedToNetworkDueToMtFunctionalityRestrictions,
    ModemNotAllowedMtRestrictedToEmergencyCallsOnly,
    OperationNotAllowedBecauseOfMtFunctionalityRestrictions,
    FixedDialNumberOnlyAllowedCalledNumberIsNotAFixedDialNumber,
    TemporarilyOutOfServiceDueToOtherMtUsage,
    Unknown,
    IllegalMs,
    IllegalMe,
    GprsServicesNotAllowed,
    GprsAndNonGprsServicesNotAllowed,
    PlmnNotAllowed,
    LocationAreaNotAllowed,
    RoamingNotAllowedInThisLocationArea,
    GprsServicesNotAllowedInThisPlmn,
    NoSuitableCellsInLocationArea,
    Congestion,
    NotAuthorizedForThisCsg,
    InsufficientResources,
    MissingOrUnknownApn,
    UnknownPdpAddressOrPdpType,
    UserAuthenticationFailed,
    RequestRejectedByServingGwOrPdnGw,
    RequestRejectedUnspecified,
    ServiceOptionNotSupported,
    RequestedServiceOptionNotSubscribed,
    ServiceOptionTemporarilyOutOfOrder,
    NsApiAlreadyUsed,
    EpsQoSNotAccepted,
    NetworkFailure,
    FeatureNotSupported,
    SemanticErrorInTheTftOperation,
    SyntacticalErrorInTheTftOperation,
    UnknownPdpContext,
    SemanticErrorsInPacketFilterS,
    SyntacticalErrorsInPacketFilterS,
    PdpContextWithoutTftAlreadyActivated,
    PtiMismatch,
    UnspecifiedGprsError,
    PdpAuthenticationFailure,
    InvalidMobileClass,
    EsmInformationNotReceived,
    PdnConnectionDoesNotExist,
    MultiplePdnConnectionsForAGivenApnNotAllowed,
    UserBusy,
    UplinkBusyFlowControl,
    BearerHandlingNotSupported,
    MaximumNumberOfEpsBearersReached,
    RequestedApnNotSupportedInCurrentRatAndPlmnCombination,
    ImsiUnknownInVlr,
    LastPdnDisconnectionNotAllowed,
    SemanticallyIncorrectMessage,
    MandatoryInformationElementError,
    InformationElementNonExistentOrNotImplemented,
    ConditionalIeError,
    ProtocolErrorUnspecified,
    OperatorDeterminedBarring,
    MaximumNumberOfPdpContextsReached,
    RequestRejectedBearerControlModeViolation,
    InvalidPtiValue,
    InvalidMandatoryIe,
    MessageTypeNonExistent,
    MessageTypeNotCompatible,
    IeNonExistent,
    MessageNotCompatible,
    InvalidErrorMapping,
    InternalError,
    SimBlocked,
    MeFailure,
    SmsServiceOfMeReserved,
    InvalidPduModeParameter,
    InvalidTextModeParameter,
    USimNotInserted,
    USimPinRequired,
    PhUSimPinRequired,
    USimFailure,
    USimBusy,
    USimWrong,
    USimPukRequired,
    USimPin2Required,
    USimPuk2Required,
    InvalidMemoryIndex,
    SmscAddressUnknown,
    NoCnmaAcknowledgementExpected,
    UnknownError,
    VoiceCallActive,
    IncorrectSecurityCode,
    MaxAttemptsReached,
    UnassignedUnallocatedNumber,
    NoRouteToDestination,
    ChannelUnacceptable,
    NormalCallClearing,
    NoUserResponding,
    UserAlertingNoAnswer,
    CallRejected,
    NumberChanged,
    NonSelectedUserClearing,
    DestinationOutOfOrder,
    InvalidNumberFormatIncompleteNumber,
    FacilityRejected,
    ResponseToStatusEnquiry,
    NormalUnspecified,
    NoCircuitChannelAvailable,
    NetworkOutOfOrder,
    TemporaryFailure,
    SwitchingEquipmentCongestion,
    AccessInformationDiscarded,
    RequestedCircuitChannelNotAvailable,
    ResourcesUnavailableUnspecified,
    QualityOfServiceUnavailable,
    RequestedFacilityNotSubscribed,
    IncomingCallsBarredWithinTheCug,
    CollisionWithNetworkInitiatedRequest,
    BearerCapabilityNotAuthorized,
    BearerCapabilityNotPresentlyAvailable,
    UnsupportedQciValue,
    ServiceOrOptionNotAvailableUnspecified,
    BearerServiceNotImplemented,
    AcmEqualToOrGreaterThanAcMmax,
    RequestedFacilityNotImplemented,
    OnlyRestrictedDigitalInformationBearerCapabilityIsAvailable,
    ServiceOrOptionNotImplementedUnspecified,
    InvalidTransactionIdentifierValue,
    UserNotMemberOfCug,
    IncompatibleDestination,
    InvalidTransitNetworkSelection,
    InvalidMandatoryInformation,
    MessageTypeNonExistentOrNotImplemented,
    MessageTypeNotCompatibleWithProtocolState,
    MessageNotCompatibleWithProtocolState,
    RecoveryOnTimerExpiry,
    ApnRestrictionValueIncompatibleWithActiveEpsBearerContext,
    InterworkingUnspecified,
    NetworkError,
    InvalidEpsBearerIdentity,
    EmmErrorUnspecified,
    EsmErrorUnspecified,
    NumberNotAllowed,
    CcbsPossible,
    WrongGpioIdentifier,
    SetGpioDefaultError,
    SelectGpioModeError,
    ReadGpioError,
    WriteGpioError,
    GpioBusy,
    WrongAdcIdentifier,
    ReadAdcError,
    IPv4OnlyAllowed,
    IPv6OnlyAllowed,
    WrongRingerIdentifier,
    LlcOrSndcpFailure,
    RegularDeactivation,
    ReactivationRequested,
    SingleAddressBearersOnlyAllowed,
    ApnRestrictionValIncompatibleWithPdpContext,
    PdpActivationRejected,
    GprsGenericOperationError,
    GprsInvalidApn,
    GprsAuthenticationFailure,
    GprsQoSParametersInconsistent,
    GprsNetworkFailure,
    GprsContextBusy,
    CsdGenericOperationError,
    CsdUndefinedProfile,
    CsdContextBusy,
    PlmnScanNotAllowed,
    FfsError,
    PdpTypeIPv4OnlyAllowed,
    PdpTypeIPv6OnlyAllowed,
    FileNotFound,
    CannotOpenFile,
    TacValueNotAllowed,
    OtpFailure,
    WrongCheckDigit,
    BufferFull,
    FfsInitializing,
    FfsAlreadyOpenFile,
    FfsNotOpenFile,
    FfsFileNotFound,
    FfsFileAlreadyCreated,
    FfsIllegalId,
    FfsIllegalFileHandle,
    FfsIllegalType,
    FfsIllegalMode,
    FfsFileRange,
    FfsOperationNotPossible,
    FfsWriteError,
    FfsUserIdError,
    FfsInternalFatalError,
    FfsMemoryResourceError,
    FfsMaximumNumberOfFilesExceeded,
    FfsMemoryNotAvailable,
    FfsInvalidFilename,
    FfsStreamingNotEnabled,
    FfsOperationNotAllowedOnStaticFile,
    FfsMemoryTableInconsistency,
    FfsNotAFactoryDefaultFile,
    FfsRequestedMemoryTemporaryNotAvailable,
    FfsOperationNotAllowedForADirectory,
    FfsDirectorySpaceNotAvailable,
    FfsTooManyStreamingFilesOpen,
    FfsRequestedDynamicMemoryTemporaryNotAvailable,
    FfsUserProvidedANullParameterInsteadOfASuitableBuffer,
    FfsTimeout,
    // CommandLineTooLong,
    // CallBarredFixedDialingNumbersOnly,
    // SecRemoteObjectWrongState,
    // SecRotNotPersonalized,
    // SecLossOfConnectivity,
    // SecServiceNotAuthorized,
    // SecFwPackageInstallationRequired,
    // SecFwPackageNotValid,
    // SecResourceNotAvailable,
    // SecDataNotAvailable,
    // SecTimeout,
    // SecDataInconsistentOrUnsupported,
    // SecPspkLockPending,
    // SecC2CAlreadyPaired,
    // SecC2CChannelsConsumed,
    // SecC2CPairingNotPresent,
    // SecBusy,
    // GpsGpioNotConfigured,
    // GpsGpioOwnershipError,
    // InvalidOperationWithGpsOn,
    // InvalidOperationWithGpsOff,
    // InvalidGpsAidingMode,
    // ReservedGpsAidingMode,
    // GpsAidingModeAlreadySet,
    // InvalidGpsTraceMode,
    // ParameterValidOnlyInCaseOfGpsOta,
    // GpsTraceInvalidServer,
    // InvalidTimeZone,
    // InvalidValue,
    // InvalidParameter,
    // InvalidOperationWithLocRunningGpsBusy,
    // NoOngoingCall,
    // IbmBusyECallAlreadyArmedActive,
    // IbmFeatureOffECallFeatureOff,
    // WrongIbmRequested,
    // AudioResourceNotAvailable,
    // EcallRestriction,
    // ECallInvalidDialNumber,
    // NoSapServerConnection,
    // SapProtocolError,
    // SapConnectionFailure,
    // SapServerDisconnection,
    // SapOtherTerminalUsingService,
    // UsecmngImportTimeoutExpired,
    // UsecmngImportFileSizeExceedsLimit,
    // UsecmngNoMemoryAvailable,
    // UsecmngInvalidCertificateKeyFormat,
    // UsecmngDatabaseFull,
    // CdcEcmIsNotAvailable,
    // CdcEcmIsBusy,
    // NoDhcpPacketsReceivedFromTheDte,
    // CommandTimeout,
    // CommandAborted,
    // ApnConfigurationMismatch,
    // IpTypeConfigurationMismatch,
    // FotaPackageDownloadStateOrNameMismatch,
    // FotaPackageDataCorrupted,
    // FotaMemoryIsInUse,
}

impl FromStr for CmeError {
    // This error will always get mapped to `atat::Error::Parse`
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            "0" | "Phone failure" => Self::PhoneFailure,
            "1" | "No connection to phone" => Self::NoConnectionToPhone,
            "2" | "Phone-adaptor link reserved" => Self::PhoneAdaptorLinkReserved,
            "3" | "302" | "Operation not allowed" => Self::OperationNotAllowed,
            "4" | "303" | "Operation not supported" => Self::OperationNotSupported,
            "5" | "PH-SIM PIN required" => Self::PhSimPinRequired,
            "6" | "PH-FSIM PIN required" => Self::PhFsimPinRequired,
            "7" | "PH-FSIM PUK required" => Self::PhFsimPukRequired,
            "10" | "SIM not inserted" => Self::SimNotInserted,
            "11" | "SIM PIN required" => Self::SimPinRequired,
            "12" | "SIM PUK required" => Self::SimPukRequired,
            "13" | "SIM failure" => Self::SimFailure,
            "14" | "SIM busy" => Self::SimBusy,
            "15" | "SIM wrong" => Self::SimWrong,
            "16" | "Incorrect password" => Self::IncorrectPassword,
            "17" | "SIM PIN2 required" => Self::SimPin2Required,
            "18" | "SIM PUK2 required" => Self::SimPuk2Required,
            "20" | "322" | "Memory full" => Self::MemoryFull,
            "21" | "Invalid index" => Self::InvalidIndex,
            "22" | "Network not found" => Self::NetworkNotFound,
            "23" | "320" | "Memory failure" => Self::MemoryFailure,
            "24" | "Text string too long" => Self::TextStringTooLong,
            "25" | "Invalid characters in text string" => Self::InvalidCharactersInTextString,
            "26" | "Dial string too long" => Self::DialStringTooLong,
            "27" | "Invalid characters in dial string" => Self::InvalidCharactersInDialString,
            "30" | "331" | "No network service" => Self::NoNetworkService,
            "31" | "332" | "Network timeout" => Self::NetworkTimeout,
            "32" | "Network not allowed - emergency calls only" => {
                Self::NetworkNotAllowedEmergencyCallsOnly
            }
            "40" | "Network personalisation PIN required" => {
                Self::NetworkPersonalisationPinRequired
            }
            "41" | "Network personalisation PUK required" => {
                Self::NetworkPersonalisationPukRequired
            }
            "42" | "Network subset personalisation PIN required" => {
                Self::NetworkSubsetPersonalisationPinRequired
            }
            "43" | "Network subset personalisation PUK required" => {
                Self::NetworkSubsetPersonalisationPukRequired
            }
            "44" | "Service provider personalisation PIN required" => {
                Self::ServiceProviderPersonalisationPinRequired
            }
            "45" | "Service provider personalisation PUK required" => {
                Self::ServiceProviderPersonalisationPukRequired
            }
            "46" | "Corporate personalisation PIN required" => {
                Self::CorporatePersonalisationPinRequired
            }
            "47" | "Corporate personalisation PUK required" => {
                Self::CorporatePersonalisationPukRequired
            }
            "50" | "Incorrect parameters" => Self::IncorrectParameters,
            "51" | "Command implemented but currently disabled" => {
                Self::CommandImplementedButCurrentlyDisabled
            }
            "52" | "Command aborted by user" => Self::CommandAbortedByUser,
            "53" | "Not attached to network due to MT functionality restrictions" => {
                Self::NotAttachedToNetworkDueToMtFunctionalityRestrictions
            }
            "54" | "Modem not allowed - MT restricted to emergency calls only" => {
                Self::ModemNotAllowedMtRestrictedToEmergencyCallsOnly
            }
            "55" | "Operation not allowed because of MT functionality restrictions" => {
                Self::OperationNotAllowedBecauseOfMtFunctionalityRestrictions
            }
            "56" | "Fixed dial number only allowed - called number is not a fixed dial number" => {
                Self::FixedDialNumberOnlyAllowedCalledNumberIsNotAFixedDialNumber
            }
            "57" | "Temporarily out of service due to other MT usage" => {
                Self::TemporarilyOutOfServiceDueToOtherMtUsage
            }
            "100" | "Unknown" => Self::Unknown,
            "103" | "Illegal MS" => Self::IllegalMs,
            "106" | "Illegal ME" => Self::IllegalMe,
            "107" | "GPRS services not allowed" => Self::GprsServicesNotAllowed,
            "108" | "GPRS and non GPRS services not allowed" => {
                Self::GprsAndNonGprsServicesNotAllowed
            }
            "111" | "PLMN not allowed" => Self::PlmnNotAllowed,
            "112" | "Location area not allowed" => Self::LocationAreaNotAllowed,
            "113" | "Roaming not allowed in this location area" => {
                Self::RoamingNotAllowedInThisLocationArea
            }
            "114" | "GPRS services not allowed in this PLMN" => {
                Self::GprsServicesNotAllowedInThisPlmn
            }
            "115" | "No Suitable Cells In Location Area" => Self::NoSuitableCellsInLocationArea,
            "122" | "170" | "Congestion" => Self::Congestion,
            "125" | "Not authorized for this CSG" => Self::NotAuthorizedForThisCsg,
            "126" | "Insufficient resources" => Self::InsufficientResources,
            "127" | "Missing or unknown APN" => Self::MissingOrUnknownApn,
            "128" | "1549" | "Unknown PDP address or PDP type" => Self::UnknownPdpAddressOrPdpType,
            "129" | "User authentication failed" => Self::UserAuthenticationFailed,
            "130" | "Request rejected by Serving GW or PDN GW" => {
                Self::RequestRejectedByServingGwOrPdnGw
            }
            "131" | "Request rejected, unspecified" => Self::RequestRejectedUnspecified,
            "132" | "Service option not supported" => Self::ServiceOptionNotSupported,
            "133" | "Requested service option not subscribed" => {
                Self::RequestedServiceOptionNotSubscribed
            }
            "134" | "Service option temporarily out of order" => {
                Self::ServiceOptionTemporarilyOutOfOrder
            }
            "135" | "NS-api already used" => Self::NsApiAlreadyUsed,
            "137" | "EPS QoS not accepted" => Self::EpsQoSNotAccepted,
            "138" | "168" | "Network failure" => Self::NetworkFailure,
            "140" | "Feature not supported" => Self::FeatureNotSupported,
            "141" | "Semantic error in the TFT operation" => Self::SemanticErrorInTheTftOperation,
            "142" | "Syntactical error in the TFT operation" => {
                Self::SyntacticalErrorInTheTftOperation
            }
            "143" | "Unknown PDP context" => Self::UnknownPdpContext,
            "144" | "Semantic errors in packet filter(s)" => Self::SemanticErrorsInPacketFilterS,
            "145" | "Syntactical errors in packet filter(s)" => {
                Self::SyntacticalErrorsInPacketFilterS
            }
            "146" | "PDP context without TFT already activated" => {
                Self::PdpContextWithoutTftAlreadyActivated
            }
            "147" | "PTI mismatch" => Self::PtiMismatch,
            "148" | "Unspecified GPRS error" => Self::UnspecifiedGprsError,
            "149" | "PDP authentication failure" => Self::PdpAuthenticationFailure,
            "150" | "Invalid mobile class" => Self::InvalidMobileClass,
            "153" | "ESM information not received" => Self::EsmInformationNotReceived,
            "154" | "PDN connection does not exist" => Self::PdnConnectionDoesNotExist,
            "155" | "Multiple PDN connections for a given APN not allowed" => {
                Self::MultiplePdnConnectionsForAGivenApnNotAllowed
            }
            "156" | "1017" | "User Busy" => Self::UserBusy,
            "159" | "Uplink Busy/ Flow Control" => Self::UplinkBusyFlowControl,
            "160" | "Bearer handling not supported" => Self::BearerHandlingNotSupported,
            "165" | "Maximum number of EPS bearers reached" => {
                Self::MaximumNumberOfEpsBearersReached
            }
            "166" | "179" | "Requested APN not supported in current RAT and PLMN combination" => {
                Self::RequestedApnNotSupportedInCurrentRatAndPlmnCombination
            }
            "169" | "IMSI unknown in VLR" => Self::ImsiUnknownInVlr,
            "171" | "1149" | "Last PDN disconnection not allowed" => {
                Self::LastPdnDisconnectionNotAllowed
            }
            "172" | "189" | "1095" | "Semantically incorrect message" => {
                Self::SemanticallyIncorrectMessage
            }
            "173" | "Mandatory information element error" => Self::MandatoryInformationElementError,
            "174" | "1099" | "Information element non-existent or not implemented" => {
                Self::InformationElementNonExistentOrNotImplemented
            }
            "175" | "194" | "1100" | "Conditional IE error" => Self::ConditionalIeError,
            "176"
            | "197"
            | "1111"
            | "Protocol error, unspecified"
            | "Protocol error unspecified" => Self::ProtocolErrorUnspecified,
            "177" | "1008" | "Operator determined barring" => Self::OperatorDeterminedBarring,
            "178" | "Maximum number of PDP contexts reached" => {
                Self::MaximumNumberOfPdpContextsReached
            }
            "180" | "Request rejected, bearer control mode violation" => {
                Self::RequestRejectedBearerControlModeViolation
            }
            "181" | "Invalid PTI value" => Self::InvalidPtiValue,
            "190" | "Invalid mandatory IE" => Self::InvalidMandatoryIe,
            "191" | "Message type non existent" => Self::MessageTypeNonExistent,
            "192" | "Message type not compatible" => Self::MessageTypeNotCompatible,
            "193" | "IE non existent" => Self::IeNonExistent,
            "195" | "Message not compatible" => Self::MessageNotCompatible,
            "254" | "Invalid error mapping" => Self::InvalidErrorMapping,
            "255" | "Internal error" => Self::InternalError,
            "262" | "SIM blocked" => Self::SimBlocked,
            "300" | "ME failure" => Self::MeFailure,
            "301" | "SMS service of ME reserved" => Self::SmsServiceOfMeReserved,
            "304" | "Invalid PDU mode parameter" => Self::InvalidPduModeParameter,
            "305" | "Invalid text mode parameter" => Self::InvalidTextModeParameter,
            "310" | "(U)SIM not inserted" => Self::USimNotInserted,
            "311" | "(U)SIM PIN required" => Self::USimPinRequired,
            "312" | "PH-(U)SIM PIN required" => Self::PhUSimPinRequired,
            "313" | "(U)SIM failure" => Self::USimFailure,
            "314" | "(U)SIM busy" => Self::USimBusy,
            "315" | "(U)SIM wrong" => Self::USimWrong,
            "316" | "(U)SIM PUK required" => Self::USimPukRequired,
            "317" | "(U)SIM PIN2 required" => Self::USimPin2Required,
            "318" | "(U)SIM PUK2 required" => Self::USimPuk2Required,
            "321" | "Invalid memory index" => Self::InvalidMemoryIndex,
            "330" | "SMSC address unknown" => Self::SmscAddressUnknown,
            "340" | "No +CNMA acknowledgement expected" => Self::NoCnmaAcknowledgementExpected,
            "500" | "Unknown error" => Self::UnknownError,
            "608" | "Voice call active" => Self::VoiceCallActive,
            "701" | "Incorrect security code" => Self::IncorrectSecurityCode,
            "702" | "Max attempts reached" => Self::MaxAttemptsReached,
            "1001" | "Unassigned (unallocated) number" => Self::UnassignedUnallocatedNumber,
            "1003" | "No route to destination" => Self::NoRouteToDestination,
            "1006" | "Channel unacceptable" => Self::ChannelUnacceptable,
            "1016" | "Normal call clearing" => Self::NormalCallClearing,
            "1018" | "No user responding" => Self::NoUserResponding,
            "1019" | "User alerting, no answer" => Self::UserAlertingNoAnswer,
            "1021" | "Call rejected" => Self::CallRejected,
            "1022" | "Number changed" => Self::NumberChanged,
            "1026" | "Non selected user clearing" => Self::NonSelectedUserClearing,
            "1027" | "Destination out of order" => Self::DestinationOutOfOrder,
            "1028" | "Invalid number format (incomplete number)" => {
                Self::InvalidNumberFormatIncompleteNumber
            }
            "1029" | "Facility rejected" => Self::FacilityRejected,
            "1030" | "Response to STATUS ENQUIRY" => Self::ResponseToStatusEnquiry,
            "1031" | "Normal, unspecified" => Self::NormalUnspecified,
            "1034" | "No circuit/channel available" => Self::NoCircuitChannelAvailable,
            "1038" | "Network out of order" => Self::NetworkOutOfOrder,
            "1041" | "Temporary failure" => Self::TemporaryFailure,
            "1042" | "Switching equipment congestion" => Self::SwitchingEquipmentCongestion,
            "1043" | "Access information discarded" => Self::AccessInformationDiscarded,
            "1044" | "requested circuit/channel not available" => {
                Self::RequestedCircuitChannelNotAvailable
            }
            "1047" | "Resources unavailable, unspecified" => Self::ResourcesUnavailableUnspecified,
            "1049" | "Quality of service unavailable" => Self::QualityOfServiceUnavailable,
            "1050" | "Requested facility not subscribed" => Self::RequestedFacilityNotSubscribed,
            "1055" | "Incoming calls barred within the CUG" => {
                Self::IncomingCallsBarredWithinTheCug
            }
            "1056" | "Collision with network initiated request" => {
                Self::CollisionWithNetworkInitiatedRequest
            }
            "1057" | "Bearer capability not authorized" => Self::BearerCapabilityNotAuthorized,
            "1058" | "Bearer capability not presently available" => {
                Self::BearerCapabilityNotPresentlyAvailable
            }
            "1059" | "Unsupported QCI value" => Self::UnsupportedQciValue,
            "1063" | "Service or option not available, unspecified" => {
                Self::ServiceOrOptionNotAvailableUnspecified
            }
            "1065" | "Bearer service not implemented" => Self::BearerServiceNotImplemented,
            "1068" | "ACM equal to or greater than ACMmax" => Self::AcmEqualToOrGreaterThanAcMmax,
            "1069" | "Requested facility not implemented" => Self::RequestedFacilityNotImplemented,
            "1070" | "Only restricted digital information bearer capability is available" => {
                Self::OnlyRestrictedDigitalInformationBearerCapabilityIsAvailable
            }
            "1079" | "Service or option not implemented, unspecified" => {
                Self::ServiceOrOptionNotImplementedUnspecified
            }
            "1081" | "1546" | "Invalid transaction identifier value" => {
                Self::InvalidTransactionIdentifierValue
            }
            "1087" | "User not member of CUG" => Self::UserNotMemberOfCug,
            "1088" | "Incompatible destination" => Self::IncompatibleDestination,
            "1091" | "Invalid transit network selection" => Self::InvalidTransitNetworkSelection,
            "1096" | "Invalid mandatory information" => Self::InvalidMandatoryInformation,
            "1097" | "Message type non-existent or not implemented" => {
                Self::MessageTypeNonExistentOrNotImplemented
            }
            "1098" | "Message type not compatible with protocol state" => {
                Self::MessageTypeNotCompatibleWithProtocolState
            }
            "1101" | "Message not compatible with protocol state" => {
                Self::MessageNotCompatibleWithProtocolState
            }
            "1102" | "Recovery on timer expiry" => Self::RecoveryOnTimerExpiry,
            "1112" | "APN restriction value incompatible with active EPS bearer context" => {
                Self::ApnRestrictionValueIncompatibleWithActiveEpsBearerContext
            }
            "1127" | "Interworking, unspecified" => Self::InterworkingUnspecified,
            "1142" | "Network Error" => Self::NetworkError,
            "1143" | "Invalid EPS bearer identity" => Self::InvalidEpsBearerIdentity,
            "1243" | "Emm Error Unspecified" => Self::EmmErrorUnspecified,
            "1244" | "Esm Error Unspecified" => Self::EsmErrorUnspecified,
            "1279" | "Number not allowed" => Self::NumberNotAllowed,
            "1283" | "CCBS possible" => Self::CcbsPossible,
            "1500" | "Wrong GPIO identifier" => Self::WrongGpioIdentifier,
            "1501" | "Set GPIO default error" => Self::SetGpioDefaultError,
            "1502" | "Select GPIO mode error" => Self::SelectGpioModeError,
            "1503" | "Read GPIO error" => Self::ReadGpioError,
            "1504" | "Write GPIO error" => Self::WriteGpioError,
            "1505" | "GPIO busy" => Self::GpioBusy,
            "1520" | "Wrong ADC identifier" => Self::WrongAdcIdentifier,
            "1521" | "Read ADC error" => Self::ReadAdcError,
            "1530" | "IPv4 only allowed" => Self::IPv4OnlyAllowed,
            "1531" | "IPv6 only allowed" => Self::IPv6OnlyAllowed,
            "1540" | "Wrong ringer identifier" => Self::WrongRingerIdentifier,
            "1542" | "LLC or SNDCP failure" => Self::LlcOrSndcpFailure,
            "1543" | "Regular deactivation" => Self::RegularDeactivation,
            "1544" | "Reactivation requested" => Self::ReactivationRequested,
            "1545" | "Single address bearers only allowed" => Self::SingleAddressBearersOnlyAllowed,
            "1547" | "APN restriction val incompatible with PDP context" => {
                Self::ApnRestrictionValIncompatibleWithPdpContext
            }
            "1548" | "PDP activation rejected" => Self::PdpActivationRejected,
            "1550" | "GPRS generic operation error" => Self::GprsGenericOperationError,
            "1551" | "GPRS invalid APN" => Self::GprsInvalidApn,
            "1552" | "GPRS authentication failure" => Self::GprsAuthenticationFailure,
            "1553" | "GPRS QoS parameters inconsistent" => Self::GprsQoSParametersInconsistent,
            "1554" | "GPRS network failure" => Self::GprsNetworkFailure,
            "1555" | "GPRS context busy" => Self::GprsContextBusy,
            "1556" | "CSD generic operation error" => Self::CsdGenericOperationError,
            "1557" | "CSD undefined profile" => Self::CsdUndefinedProfile,
            "1558" | "CSD context busy" => Self::CsdContextBusy,
            "1559" | "PLMN scan not allowed" => Self::PlmnScanNotAllowed,
            "1600" | "FFS error" => Self::FfsError,
            "1560" | "PDP type IPv4 only allowed" => Self::PdpTypeIPv4OnlyAllowed,
            "1561" | "PDP type IPv6 only allowed" => Self::PdpTypeIPv6OnlyAllowed,
            "1612" | "FILE NOT FOUND" => Self::FileNotFound,
            "1613" | "Cannot open file" => Self::CannotOpenFile,
            "1614" | "TAC value not allowed" => Self::TacValueNotAllowed,
            "1615" | "OTP failure" => Self::OtpFailure,
            "1616" | "Wrong Check Digit" => Self::WrongCheckDigit,
            "1620" | "Buffer full" => Self::BufferFull,
            "1621" | "FFS initializing" => Self::FfsInitializing,
            "1622" | "FFS already open file" => Self::FfsAlreadyOpenFile,
            "1623" | "FFS not open file" => Self::FfsNotOpenFile,
            "1624" | "FFS file not found" => Self::FfsFileNotFound,
            "1625" | "FFS file already created" => Self::FfsFileAlreadyCreated,
            "1626" | "FFS illegal id" => Self::FfsIllegalId,
            "1627" | "FFS illegal file handle" => Self::FfsIllegalFileHandle,
            "1628" | "FFS illegal type" => Self::FfsIllegalType,
            "1629" | "FFS illegal mode" => Self::FfsIllegalMode,
            "1630" | "FFS file range" => Self::FfsFileRange,
            "1631" | "FFS operation not possible" => Self::FfsOperationNotPossible,
            "1632" | "FFS write error" => Self::FfsWriteError,
            "1633" | "FFS user id error" => Self::FfsUserIdError,
            "1634" | "FFS internal fatal error" => Self::FfsInternalFatalError,
            "1635" | "FFS memory resource error" => Self::FfsMemoryResourceError,
            "1636" | "FFS maximum number of files exceeded" => {
                Self::FfsMaximumNumberOfFilesExceeded
            }
            "1637" | "FFS memory not available" => Self::FfsMemoryNotAvailable,
            "1638" | "FFS invalid filename" => Self::FfsInvalidFilename,
            "1639" | "FFS streaming not enabled" => Self::FfsStreamingNotEnabled,
            "1640" | "FFS operation not allowed on static file" => {
                Self::FfsOperationNotAllowedOnStaticFile
            }
            "1641" | "FFS memory table inconsistency" => Self::FfsMemoryTableInconsistency,
            "1642" | "FFS not a factory default file" => Self::FfsNotAFactoryDefaultFile,
            "1643" | "FFS requested memory temporary not available" => {
                Self::FfsRequestedMemoryTemporaryNotAvailable
            }
            "1644" | "FFS operation not allowed for a directory" => {
                Self::FfsOperationNotAllowedForADirectory
            }
            "1645" | "FFS directory space not available" => Self::FfsDirectorySpaceNotAvailable,
            "1646" | "FFS too many streaming files open" => Self::FfsTooManyStreamingFilesOpen,
            "1647" | "FFS requested dynamic memory temporary not available" => {
                Self::FfsRequestedDynamicMemoryTemporaryNotAvailable
            }
            "1648" | "FFS user provided a NULL parameter instead of a suitable buffer" => {
                Self::FfsUserProvidedANullParameterInsteadOfASuitableBuffer
            }
            "1649" | "FFS timeout" => Self::FfsTimeout,
            // "1650" | "Command line too long" => Self::CommandLineTooLong,
            // "1660" | "Call barred - Fixed dialing numbers only" => {
            //     Self::CallBarredFixedDialingNumbersOnly
            // }
            // "1670" | "SEC remote object wrong state" => Self::SecRemoteObjectWrongState,
            // "1671" | "SEC ROT not personalized" => Self::SecRotNotPersonalized,
            // "1672" | "SEC loss of connectivity" => Self::SecLossOfConnectivity,
            // "1673" | "SEC service not authorized" => Self::SecServiceNotAuthorized,
            // "1674" | "SEC FW package installation required" => {
            //     Self::SecFwPackageInstallationRequired
            // }
            // "1675" | "SEC FW package not valid" => Self::SecFwPackageNotValid,
            // "1676" | "SEC resource not available" => Self::SecResourceNotAvailable,
            // "1677" | "SEC data not available" => Self::SecDataNotAvailable,
            // "1678" | "SEC timeout" => Self::SecTimeout,
            // "1679" | "SEC data inconsistent or unsupported" => {
            //     Self::SecDataInconsistentOrUnsupported
            // }
            // "1680" | "SEC pspk lock pending" => Self::SecPspkLockPending,
            // "1681" | "SEC C2C already paired" => Self::SecC2CAlreadyPaired,
            // "1682" | "SEC C2C channels consumed" => Self::SecC2CChannelsConsumed,
            // "1683" | "SEC C2C pairing not present" => Self::SecC2CPairingNotPresent,
            // "1684" | "SEC busy" => Self::SecBusy,
            // "1700" | "GPS GPIO not configured" => Self::GpsGpioNotConfigured,
            // "1701" | "GPS GPIO ownership error" => Self::GpsGpioOwnershipError,
            // "1702" | "Invalid operation with GPS ON" => Self::InvalidOperationWithGpsOn,
            // "1703" | "Invalid operation with GPS OFF" => Self::InvalidOperationWithGpsOff,
            // "1704" | "Invalid GPS aiding mode" => Self::InvalidGpsAidingMode,
            // "1705" | "Reserved GPS aiding mode" => Self::ReservedGpsAidingMode,
            // "1706" | "GPS aiding mode already set" => Self::GpsAidingModeAlreadySet,
            // "1707" | "Invalid GPS trace mode" => Self::InvalidGpsTraceMode,
            // "1708" | "Parameter valid only in case of GPS OTA" => {
            //     Self::ParameterValidOnlyInCaseOfGpsOta
            // }
            // "1709" | "GPS trace invalid server" => Self::GpsTraceInvalidServer,
            // "1710" | "Invalid TimeZone" => Self::InvalidTimeZone,
            // "1711" | "Invalid value" => Self::InvalidValue,
            // "1712" | "Invalid parameter" => Self::InvalidParameter,
            // "1713" | "Invalid operation with LOC running / GPS Busy" => {
            //     Self::InvalidOperationWithLocRunningGpsBusy
            // }
            // "1800" | "No ongoing call" => Self::NoOngoingCall,
            // "1801" | "IBM busy / eCall already armed/active" => {
            //     Self::IbmBusyECallAlreadyArmedActive
            // }
            // "1802" | "IBM feature off / eCall feature off" => Self::IbmFeatureOffECallFeatureOff,
            // "1803" | "Wrong IBM requested" => Self::WrongIbmRequested,
            // "1804" | "Audio resource not available" => Self::AudioResourceNotAvailable,
            // "1805" | "ECALL restriction" => Self::EcallRestriction,
            // "1806" | "eCall invalid dial number" => Self::ECallInvalidDialNumber,
            // "1900" | "No SAP Server Connection" => Self::NoSapServerConnection,
            // "1901" | "SAP Protocol Error" => Self::SapProtocolError,
            // "1902" | "SAP Connection failure" => Self::SapConnectionFailure,
            // "1903" | "SAP Server Disconnection" => Self::SapServerDisconnection,
            // "1904" | "SAP Other terminal using service" => Self::SapOtherTerminalUsingService,
            // "1910" | "USECMNG import timeout expired (no input for > 20 s)" => {
            //     Self::UsecmngImportTimeoutExpired
            // }
            // "1911" | "USECMNG import file size exceeds limit" => {
            //     Self::UsecmngImportFileSizeExceedsLimit
            // }
            // "1912" | "USECMNG no memory available" => Self::UsecmngNoMemoryAvailable,
            // "1913" | "USECMNG invalid certificate/key format" => {
            //     Self::UsecmngInvalidCertificateKeyFormat
            // }
            // "1914" | "USECMNG database full" => Self::UsecmngDatabaseFull,
            // "1950" | "CDC-ECM is not available" => Self::CdcEcmIsNotAvailable,
            // "1951" | "CDC-ECM is busy" => Self::CdcEcmIsBusy,
            // "1952" | "No DHCP Packets received from the DTE" => {
            //     Self::NoDhcpPacketsReceivedFromTheDte
            // }
            // "2000" | "Command timeout" => Self::CommandTimeout,
            // "3000" | "Command aborted" => Self::CommandAborted,
            // "4000" | "APN configuration mismatch" => Self::ApnConfigurationMismatch,
            // "4001" | "IP type configuration mismatch" => Self::IpTypeConfigurationMismatch,
            // "5000" | "FOTA package download state or name mismatch" => {
            //     Self::FotaPackageDownloadStateOrNameMismatch
            // }
            // "5001" | "FOTA package data corrupted" => Self::FotaPackageDataCorrupted,
            // "5002" | "FOTA memory is in use" => Self::FotaMemoryIsInUse,
            _ => return Err(()),
        })
    }
}

// Idea:

// #[derive(AtatErr)]
// #[at_err("+CME ERROR")]
// pub enum CmeError {
//     #[at_arg(0, "Phone failure")]
//     PhoneFailure,
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_error() {
        assert_eq!(
            "+CME ERROR: Something went wrong".parse::<UbloxError>(),
            Ok(UbloxError::Cme(CmeError::Unknown))
        );
        assert_eq!(
            "+CMS ERROR: Something went wrong".parse::<UbloxError>(),
            Ok(UbloxError::Cms(CmsError::Unknown))
        );
    }
    #[test]
    fn numeric_error() {
        assert_eq!(
            "+CME ERROR: 0".parse::<UbloxError>(),
            Ok(UbloxError::Cme(CmeError::PhoneFailure))
        );
        assert_eq!(
            "+CME ERROR: 1500".parse::<UbloxError>(),
            Ok(UbloxError::Cme(CmeError::WrongGpioIdentifier))
        );
    }

    #[test]
    fn verbose_error() {
        assert_eq!(
            "+CME ERROR: Phone failure".parse::<UbloxError>(),
            Ok(UbloxError::Cme(CmeError::PhoneFailure))
        );
        assert_eq!(
            "+CME ERROR: Wrong GPIO identifier".parse::<UbloxError>(),
            Ok(UbloxError::Cme(CmeError::WrongGpioIdentifier))
        );
    }

    #[test]
    fn generic_error() {
        assert_eq!("ERROR".parse::<UbloxError>(), Ok(UbloxError::Generic));
    }
}
