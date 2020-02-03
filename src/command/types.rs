use core::fmt::{self, Display};
use heapless::{consts, String};
use no_std_net::IpAddr;

#[derive(Debug, Clone, PartialEq)]
pub struct Seconds(pub u32);

impl Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Index representing a network-assigned or network-negotiated parameter:
#[derive(Debug, Clone, PartialEq)]
pub enum PacketSwitchedNetworkDataParam {
    /// • 0: IP address: dynamic IP address assigned during PDP context activation;
    IPAddress = 0,
    /// • 1: DNS1: dynamic primary DNS address;
    DNS1 = 1,
    /// • 2: DNS2: dynamic secondary DNS address;
    DNS2 = 2,
    /// • 3: QoS precedence: network assigned precedence class of the QoS;
    QoSPrecedence = 3,
    /// • 4: QoS delay: network assigned delay class of the QoS;
    QoSDelay = 4,
    /// • 5: QoS reliability: network assigned reliability class of the QoS;
    QoSReliability = 5,
    /// • 6: QoS peak rate: network assigned peak rate value of the QoS;
    QoSPeakRate = 6,
    /// • 7: QoS mean rate: network assigned mean rate value of the QoS
    QoSMeanRate = 7,
    /// • 8: PSD profile status: if the profile is active the return value is 1, 0 otherwise
    PsdProfileStatus = 8,
    /// • 9: 3G QoS delivery order
    QoS3GDeliveryOrder = 9,
    /// • 10: 3G QoS erroneous SDU delivery
    /// • 11: 3G QoS extended guaranteed downlink bit rate
    /// • 12: 3G QoS extended maximum downlink bit rate
    /// • 13: 3G QoS guaranteed downlink bit rate
    /// • 14: 3G QoS guaranteed uplink bit rate
    /// • 15: 3G QoS maximum downlink bit rate
    /// • 16: 3G QoS maximum uplink bit rate
    /// • 17: 3G QoS maximum SDU size
    /// • 18: 3G QoS residual bit error rate
    /// • 19: 3G QoS SDU error ratio
    /// • 20: 3G QoS signalling indicator
    /// • 21: 3G QoS source statistics descriptor
    /// • 22: 3G QoS traffic class
    /// • 23: 3G QoS traffic priority
    /// • 24: 3G QoS transfer delay
    QoS3GTransferDelay = 24,
}

/// GPIO output value (for output function <gpio_mode>=0 only):
#[derive(Debug, Clone, PartialEq)]
pub enum GpioOutValue {
    Low = 0,
    High = 1,
}

/// Socket protocol
#[derive(Debug, Clone, PartialEq)]
pub enum SocketProtocol {
    TCP = 6,
    UDP = 17,
}

/// GPIO input value (for input function <gpio_mode>=1 only):
#[derive(Debug, Clone, PartialEq)]
pub enum GpioInPull {
    /// (default value): no resistor activated
    NoPull = 0,
    /// pull up resistor active
    PullUp = 1,
    /// pull down resistor active
    PullDown = 2,
}

/// Mode identifier: configured function
/// See the GPIO functions for custom functions supported by different u-blox cellular
/// modules series and product version
#[derive(Debug, Clone, PartialEq)]
pub enum GpioMode {
    /// • 0: output
    Output(GpioOutValue),
    /// • 1: input
    Input(GpioInPull),
    /// • 2: network status indication
    NetworkStatus,
    /// • 3: external GNSS supply enable
    ExternalGnssSupplyEnable,
    /// • 4: external GNSS data ready
    ExternalGnssDataReady,
    /// • 5: external GNSS RTC sharing
    ExternalGnssRtcSharing,
    /// • 6: jamming detection indication
    JammingDetection,
    /// • 7: SIM card detection
    SimDetection,
    /// • 8: headset detection
    HeadsetDetection,
    /// • 9: GSM Tx burst indication
    GsmTxIndication,
    /// • 10: module operating status indication
    ModuleOperatingStatus,
    /// • 11: module functionality status indication
    ModuleFunctionalityStatus,
    /// • 12: I2S digital audio interface
    I2SDigitalAudio,
    /// • 13: SPI serial interface
    SpiSerial,
    /// • 14: master clock generation
    MasterClockGeneration,
    /// • 15: UART (DSR, DTR, DCD e RI) interface
    Uart,
    /// • 16: Wi-Fi enable
    WifiEnable,
    /// • 18: ring indication
    RingIndication,
    /// • 19: last gasp enable
    LastGaspEnable,
    /// • 20: external GNSS antenna / LNA control enable
    ExternalGnssAntenna,
    /// • 21: time pulse GNSS
    TimePulseGnss,
    /// • 22: time pulse modem
    TimePulseModem,
    /// • 23: time stamp of external interrupt
    TimestampExternalInterrupt,
    /// • 24: fast power-off
    FastPoweroff,
    /// • 25: LwM2M pulse
    Lwm2mPulse,
    /// • 26: hardware flow control (RTS, CTS)
    HardwareFlowControl,
    /// • 32: 32.768 kHz output
    ClockOutput,
    /// • 255: pad disabled
    PadDisabled,
}

impl Display for GpioMode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", match self {
            GpioMode::Output(_) => 0,
            GpioMode::Input(_) => 1,
            GpioMode::NetworkStatus => 2,
            GpioMode::ExternalGnssSupplyEnable => 3,
            GpioMode::ExternalGnssDataReady => 4,
            GpioMode::ExternalGnssRtcSharing => 5,
            GpioMode::JammingDetection => 6,
            GpioMode::SimDetection => 7,
            GpioMode::HeadsetDetection => 8,
            GpioMode::GsmTxIndication => 9,
            GpioMode::ModuleOperatingStatus => 10,
            GpioMode::ModuleFunctionalityStatus => 11,
            GpioMode::I2SDigitalAudio => 12,
            GpioMode::SpiSerial => 13,
            GpioMode::MasterClockGeneration => 14,
            GpioMode::Uart => 15,
            GpioMode::WifiEnable => 16,
            GpioMode::RingIndication => 18,
            GpioMode::LastGaspEnable => 19,
            GpioMode::ExternalGnssAntenna => 20,
            GpioMode::TimePulseGnss => 21,
            GpioMode::TimePulseModem => 22,
            GpioMode::TimestampExternalInterrupt => 23,
            GpioMode::FastPoweroff => 24,
            GpioMode::Lwm2mPulse => 25,
            GpioMode::HardwareFlowControl => 26,
            GpioMode::ClockOutput => 32,
            GpioMode::PadDisabled => 255,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PacketSwitchedAction {
    /// It clears the specified profile resetting all the parameters to their factory programmed values
    Reset = 0,
    /// It saves all the parameters in NVM
    Store = 1,
    /// It reads all the parameters from NVM
    Load = 2,
    /// It activates a PDP context with the specified profile, using the current parameters
    Activate = 3,
    /// It deactivates the PDP context associated with the specified profile
    Deactivate = 4,
}


#[derive(Debug, Clone, PartialEq)]
pub enum TerminationErrorMode {
    /// +CME ERROR: <err> result code disabled and ERROR used
    Disabled = 0,
    /// +CME ERROR: <err> result code enabled and numeric <err> values used
    Enabled = 1,
    /// +CME ERROR: <err> result code enabled and verbose <err> values used
    Verbose = 2
}

/// Reset mode. This parameter can be used only when <fun> (ModuleFunctionality) is 1, 4 or 19
#[derive(Debug, Clone, PartialEq)]
pub enum ResetMode {
    /// Do not reset the MT before setting it to the selected <fun>
    DontReset = 0,
    /// Performs a MT silent reset (with detach from network and saving of nvm
    /// parameters) with reset of the SIM card before setting it to the selected <fun>
    Reset = 1,
}

#[derive(Debug, Clone, PartialEq)]
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
#[cfg(any(
    feature = "toby_l2",
    feature = "mpci_l2",
    feature = "toby_l4"
))]
    MinimumWithoutSim = 19,

    /// Sets the MT in a deep low power state "HALT" (with detach from the network
    /// and saving of the NVM parameters); the only way to wake up the module is a power
    /// cycle or a module reset
#[cfg(any(
    feature = "toby_l2",
    feature = "mpci_l2"
))]
    Halt = 127,
}


/// Power saving configuration. Allowed values:
#[derive(Debug, Clone, PartialEq)]
pub enum PowerSavingMode {
    /// Disabled: (default and factory-programmed value)
    Disabled = 0,
    /// Enabled:
    /// The UART is re-enabled from time to time to allow the DTE to transmit, and
    /// the module switches from idle to active mode in a cyclic way. If during the
    /// active mode any data is received, the UART (and the module) is forced to stay
    /// "awake" for a time specified by the <Timeout> parameter. Any subsequent data
    /// reception during the "awake" period resets and restarts the "awake" timer
    Enabled = 1,
    /// Power saving is controlled by UART RTS line:
    /// o If the RTS line state is set to OFF, the power saving mode is allowed
    /// o If the RTS line state is set to ON, the module shall exit from power saving mode
    /// <mode>=2 is allowed only if the HW flow control has been previously disabled
    /// on the UART interface (e.g. with AT&K0), otherwise the command returns an
    /// error result code (+CME ERROR: operation not allowed if +CMEE is set to 2).
    /// With <mode>=2 the DTE can start sending data to the module without risk of
    /// data loss after having asserted the UART RTS line (RTS line set to ON state).
    CtrlByRts = 2,
    /// Power saving is controlled by UART DTR line:
    /// If the DTR line state is set to OFF, the power saving mode is allowed
    /// If the DTR line state is set to ON, the module shall exit from power saving mode
    /// <mode>=3 is allowed regardless the flow control setting on the UART
    /// interface. In particular, the HW flow control can be set on UART during this
    /// mode.
    /// With <mode>=3 the DTE can start sending data to the module without risk of
    /// data loss after having asserted the UART DTR line (DTR line set to ON state).
    CtrlByDtr = 3,
}


#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolType {
    /// (factory-programmed value): IPv4
    IPv4 = 0,
    /// IPv6
    IPv6 = 1,
    /// IPv4v6 with IPv4 preferred for internal sockets
    IPv4v6PreferV4Internal = 2,
    /// IPv4v6 with IPv6 preferred for internal sockets
    IPv4v6PreferV6Internal = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuthenticationType {
    /// (factory-programmed value): none
    None = 0,
    /// PAP
    PAP = 1,
    /// CHAP
    CHAP = 2,
    /// automatic selection of authentication type (none/CHAP/PAP)
    Auto = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataCompression {
    /// (factory-programmed value): off
    Off = 0,
    /// predefined, i.e. V.42bis
    Predefined = 1,
    /// V.42bis
    V42Bits = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeaderCompression {
    /// (factory-programmed value): off
    Off = 0,
    /// predefined, i.e. RFC1144
    Predefined = 1,
    /// RFC1144
    RFC1144 = 2,
    /// RFC2507
    RFC2507 = 3,
    /// RFC3095
    RFC3095 = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QoSPrecedence {
    /// (factory-programmed value): subscribed
    Subscribed = 0,
    /// high
    High = 1,
    /// normal
    Normal = 2,
    /// low
    Low = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QoSDelay {
    /// (factory-programmed value): subscribed
    Subscribed = 0,
    /// class 1
    Class1 = 1,
    /// class 2
    Class2 = 2,
    /// class 3
    Class3 = 3,
    /// best effort
    BestEffort = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum QoSReliability {
    /// (factory-programmed value): subscribed
    Subscribed = 0,
    /// class 1 (Interpreted as class 2)
    Class1 = 1,
    /// class 2 (GTP Unack, LLC Ack and Protected, RLC Ack)
    Class2 = 2,
    /// class 3 (GTP Unack, LLC Unack and Protected, RLC Ack)
    Class3 = 3,
    /// class 4 (GTP Unack, LLC Unack and Protected, RLC Unack)
    Class4 = 4,
    /// class 5 (GTP Unack, LLC Unack and Unprotected, RLC Unack)
    Class5 = 5,
    /// class 6 (Interpreted as class 3)
    Class6 = 6,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PacketSwitchedParam {
    /// • 0: Protocol type; the allowed values of <param_val> parameter are
    ProtocolType(ProtocolType),
    /// • 1: APN - <param_val> defines the APN text string, e.g. "apn.provider.com"; the
    /// maximum length is 99. The factory-programmed value is an empty string.
    APN(String<consts::U128>),
    /// • 2: username - <param_val> is the user name text string for the authentication
    /// phase. The factory-programmed value is an empty string.
    Username(String<consts::U128>),
    /// • 3: password - <param_val> is the password text string for the authentication phase.
    /// Note: the AT+UPSD read command with param_tag = 3 is not allowed and the read
    /// all command does not display it
    Password(String<consts::U128>),
    /// • 4: DNS1 - <param_val> is the text string of the primary DNS address. IPv4 DNS
    /// addresses are specified in dotted decimal notation form (i.e. four numbers in
    /// range 0-255 separated by periods, e.g. "xxx.yyy.zzz.www"). IPv6 DNS addresses
    /// are specified in standard IPv6 notation form (2001:DB8:: address compression is
    /// allowed). The factory-programmed value is "0.0.0.0".
    DNS1(IpAddr),
    /// • 5: DNS2 - <param_val> is the text string of the secondary DNS address. IPv4
    /// DNS addresses are specified in dotted decimal notation form (i.e. four numbers
    /// in range 0-255 separated by periods, e.g. "xxx.yyy.zzz.www"). IPv6 DNS addresses
    /// are specified in standard IPv6 notation form (2001:DB8:: address compression is
    /// allowed). The factory-programmed value is "0.0.0.0".
    DNS2(IpAddr),
    /// • 6: authentication - the <param_val> parameter selects the authentication type:
    Authentication(AuthenticationType),
    /// • 7: IP address - <param_val> is the text string of the static IP address given by the
    /// ISP in dotted decimal notation form (i.e. four numbers in range 0-255 separated by
    /// periods, e.g. "xxx.yyy.zzz.www"). The factory-programmed value is "0.0.0.0". Note:
    /// IP address set as "0.0.0.0" means dynamic IP address assigned during PDP context
    /// activation
    IPAddress(IpAddr),
    /// • 8: data compression - the <param_val> parameter refers to the default parameter
    /// named d_comp and selects the data compression type:
    DataCompression(DataCompression),
    /// • 9: header compression - the <param_val> parameter refers to the default
    /// parameter named h_comp and selects the header compression type:
    HeaderCompression(HeaderCompression),
    /// • 10: QoS precedence - the <param_val> parameter selects the precedence class:
    QoSPrecedence(QoSPrecedence),
    /// • 11: QoS delay - the <param_val> parameter selects the delay class:
    QoSDelay(QoSDelay),
    /// • 12: QoS reliability - the <param_val> parameter selects the reliability class:
    QoSReliability(QoSReliability),
    /// • 13: QoS peak rate - the <param_val> parameter selects the peak throughput in
    /// range 0-9. The factory-programmed value is 0.
    /// • 14: QoS mean rate - the <param_val> parameter selects the mean throughput in
    /// range 0-18, 31. The factory-programmed value is 0.
    /// • 15: minimum QoS precedence - the <param_val> parameter selects the acceptable
    /// value for the precedence class:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: high
    /// o 2: normal
    /// o 3: low
    /// • 16: minimum QoS delay - the <param_val> parameter selects the acceptable value
    /// for the delay class:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: class 1
    /// o 2: class 2
    /// o 3: class 3
    /// o 4: best effort
    /// • 17: minimum QoS reliability - the <param_val> parameter selects the minimum
    /// acceptable value for the reliability class:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: class 1 (Interpreted as class 2)
    /// o 2: class 2 (GTP Unack, LLC Ack and Protected, RLC Ack)
    /// o 3: class 3 (GTP Unack, LLC Unack and Protected, RLC Ack)
    /// o 4: class 4 (GTP Unack, LLC Unack and Protected, RLC Unack)
    /// o 5: class 5 (GTP Unack, LLC Unack and Unprotected, RLC Unack)
    /// o 6: class 6 (Interpreted as class 3)
    /// • 18: minimum QoS peak rate - the <param_val> parameter selects the acceptable
    /// value for the peak throughput in range 0-9. The factory-programmed value is 0.
    /// • 19: minimum QoS mean rate - the <param_val> parameter selects the acceptable
    /// value for the mean throughput in range 0-18, 31. The factory-programmed value is 0.
    /// • 20: 3G QoS delivery order - the <param_val> parameter selects the acceptable value
    /// for the delivery order:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: enable
    /// o 2: disable
    /// • 21: 3G QoS erroneous SDU delivery - the <param_val> parameter selects the
    /// acceptable value for the erroneous SDU delivery:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: no detection
    /// o 2: enable
    /// o 3: disable
    /// • 22: 3G QoS extended guaranteed downlink bit rate - <param_val> is the value for the
    /// extended guaranteed downlink bit rate in kb/s. The factory-programmed value is 0.
    /// • 23: 3G QoS extended maximum downlink bit rate - <param_val> is the value for the
    /// extended maximum downlink bit rate in kb/s. The factory-programmed value is 0.
    /// • 24: 3G QoS guaranteed downlink bit rate - <param_val> is the value for the
    /// guaranteed downlink bit rate in kb/s. The factory-programmed value is 0.
    /// • 25: 3G QoS guaranteed uplink bit rate - <param_val> is the value for the guaranteed
    /// uplink bit rate in kb/s. The factory-programmed value is 0.
    /// • 26: 3G QoS maximum downlink bit rate - <param_val> is the value for the maximum
    /// downlink bit rate in kb/s. The factory-programmed value is 0.
    /// • 27: 3G QoS maximum uplink bit rate - <param_val> is the value for the maximum
    /// uplink bit rate in kb/s. The factory-programmed value is 0.
    /// • 28: 3G QoS maximum SDU size - <param_val> is the value for the maximum SDU
    /// size in octets. The factory-programmed value is 0.
    /// • 29: 3G QoS residual bit error rate - <param_val> selects the acceptable value for the
    /// residual bit error rate:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: 5E2
    /// o 2: 1E2
    /// o 3: 5E3
    /// o 4: 4E3
    /// o 5: 1E3
    /// o 6: 1E4
    /// o 7: 1E5
    /// o 8: 1E6
    /// o 9: 6E8
    /// • 30: 3G QoS SDU error ratio - <param_val> selects the acceptable value for the SDU
    /// error ratio:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: 1E2
    /// o 2: 7E3
    /// o 3: 1E3
    /// o 4: 1E4
    /// o 5: 1E5
    /// o 6: 1E6
    /// o 7: 1E1
    /// • 31: 3G QoS signalling indicator - <param_val> selects the acceptable value for the
    /// signalling indicator:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: signalling indicator 1
    /// • 32: 3G QoS source statistics descriptor - <param_val> selects the acceptable value
    /// for the source statistics descriptor:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: source statistics descriptor 1
    /// • 33: 3G QoS traffic class - <param_val> selects the acceptable value for the traffic
    /// class:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: conversational
    /// o 2: streaming
    /// o 3: interactive
    /// o 4: background
    /// • 34: 3G QoS traffic priority - <param_val> selects the acceptable value for the traffic
    /// priority:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: priority 1
    /// o 2: priority 2
    /// o 3: priority 3
    /// • 35: 3G QoS transfer delay - <param_val> is the value for the transfer delay in
    /// milliseconds. The factory-programmed value is 0.
    /// • 36: 3G minimum QoS delivery order - <param_val> selects the acceptable value for
    /// the delivery order:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: enable
    /// o 2: disable
    /// • 37: 3G minimum QoS erroneous SDU delivery - <param_val> selects the acceptable
    /// value for the erroneous SDU delivery:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: no detection
    /// o 2: enable
    /// o 3: disable
    /// • 38: 3G minimum QoS extended guaranteed downlink bit rate - <param_val> is
    /// the value for the extended guaranteed downlink bit rate in kb/s. The factoryprogrammed value is 0.
    /// • 39: 3G minimum QoS extended maximum downlink bit rate - <param_val> is the
    /// value for the extended maximum downlink bit rate in kb/s. The factory-programmed
    /// value is 0.
    /// • 40: 3G minimum QoS guaranteed downlink bit rate - <param_val> is the value for
    /// the guaranteed downlink bit rate in kb/s. The factory-programmed value is 0.
    /// • 41: 3G minimum QoS guaranteed uplink bit rate - <param_val> is the value for the
    /// guaranteed uplink bit rate in kb/s. The factory-programmed value is 0.
    /// • 42: 3G minimum QoS maximum downlink bit rate - <param_val> is the value for the
    /// maximum downlink bit rate in kb/s. The factory-programmed value is 0.
    /// • 43: 3G minimum QoS maximum uplink bit rate - <param_val> is the value for the
    /// maximum uplink bit rate in kb/s. The factory-programmed value is 0.
    /// • 44: 3G minimum QoS maximum SDU size - <param_val> is the value for the
    /// maximum SDU size in octets. The factory-programmed value is 0.
    /// • 45: 3G minimum QoS residual bit error rate - <param_val> selects the acceptable
    /// value for the residual bit error rate:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: 5E2
    /// o 2: 1E2
    /// o 3: 5E3
    /// o 4: 4E3
    /// o 5: 1E3
    /// o 6: 1E4
    /// o 7: 1E5
    /// o 8: 1E6
    /// o 9: 6E8
    /// • 46: 3G minimum QoS SDU error ratio - <param_val> selects the acceptable value
    /// for the SDU error ratio:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: 1E2
    /// o 2: 7E3
    /// o 3: 1E3
    /// o 4: 1E4
    /// o 5: 1E5
    /// o 6: 1E6
    /// o 7: 1E1
    /// • 47: 3G minimum QoS signalling indicator - <param_val> selects the acceptable
    /// value for the signalling indicator:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: signalling indicator 1
    /// • 48: 3G minimum QoS source statistics descriptor - <param_val> selects the
    /// acceptable value for the source statistics descriptor:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: source statistics descriptor 1
    /// • 49: 3G minimum QoS traffic class - <param_val> selects the acceptable value for
    /// the traffic class:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: conversational
    /// o 2: streaming
    /// o 3: interactive
    /// o 4: background
    /// • 50: 3G minimum QoS traffic priority - <param_val> selects the acceptable value for
    /// the traffic priority:
    /// o 0 (factory-programmed value): subscribed
    /// o 1: priority 1
    /// o 2: priority 2
    /// o 3: priority 3
    /// • 51: 3G Minimum QoS transfer delay - <param_val> is the value for the transfer delay
    /// in milliseconds. The factory-programmed value is 0.
    QoSDelay3G(u32),
    /// • 100: map the +UPSD profile to the specified <cid> in the +CGDCONT table.
    /// o 0: map the current profile to default bearer PDP ID
    /// o 1: map the current profile to <cid> 1
    /// o 2: map the current profile to <cid> 2
    /// o 3: map the current profile to <cid> 3
    /// o 4: map the current profile to <cid> 4
    /// o 5: map the current profile to <cid> 5
    /// o 6: map the current profile to <cid> 6
    /// o 7: map the current profile to <cid> 7
    /// o 8: map the current profile to <cid> 8
    CurrentProfileMap(u8),
}
