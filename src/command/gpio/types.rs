use ufmt::derive::uDebug;
use serde_repr::{Serialize_repr, Deserialize_repr};
use serde::{Serialize, Deserialize};


/// GPIO output value (for output function <gpio_mode>=0 only):
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GpioOutValue {
    Low = 0,
    High = 1,
}


/// GPIO input value (for input function <gpio_mode>=1 only):
#[derive(uDebug, Clone, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum GpioInPull {
    /// (default value): no resistor activated
    NoPull = 0,
    /// pull up resistor active
    PullUp = 1,
    /// pull down resistor active
    PullDown = 2,
}

//TODO: Implement serialize and Deserialize for enum
#[derive(uDebug, Clone, PartialEq, Serialize, Deserialize)]
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

// impl core::fmt::Display for GpioMode {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         match self {
//             GpioMode::Output(v) => write!(f, "{},{}", 0, v),
//             GpioMode::Input(v) => write!(f, "{},{}", 1, v),
//             GpioMode::NetworkStatus => write!(f, "{}", 2),
//             GpioMode::ExternalGnssSupplyEnable => write!(f, "{}", 3),
//             GpioMode::ExternalGnssDataReady => write!(f, "{}", 4),
//             GpioMode::ExternalGnssRtcSharing => write!(f, "{}", 5),
//             GpioMode::JammingDetection => write!(f, "{}", 6),
//             GpioMode::SimDetection => write!(f, "{}", 7),
//             GpioMode::HeadsetDetection => write!(f, "{}", 8),
//             GpioMode::GsmTxIndication => write!(f, "{}", 9),
//             GpioMode::ModuleOperatingStatus => write!(f, "{}", 10),
//             GpioMode::ModuleFunctionalityStatus => write!(f, "{}", 11),
//             GpioMode::I2SDigitalAudio => write!(f, "{}", 12),
//             GpioMode::SpiSerial => write!(f, "{}", 13),
//             GpioMode::MasterClockGeneration => write!(f, "{}", 14),
//             GpioMode::Uart => write!(f, "{}", 15),
//             GpioMode::WifiEnable => write!(f, "{}", 16),
//             GpioMode::RingIndication => write!(f, "{}", 18),
//             GpioMode::LastGaspEnable => write!(f, "{}", 19),
//             GpioMode::ExternalGnssAntenna => write!(f, "{}", 20),
//             GpioMode::TimePulseGnss => write!(f, "{}", 21),
//             GpioMode::TimePulseModem => write!(f, "{}", 22),
//             GpioMode::TimestampExternalInterrupt => write!(f, "{}", 23),
//             GpioMode::FastPoweroff => write!(f, "{}", 24),
//             GpioMode::Lwm2mPulse => write!(f, "{}", 25),
//             GpioMode::HardwareFlowControl => write!(f, "{}", 26),
//             GpioMode::ClockOutput => write!(f, "{}", 32),
//             GpioMode::PadDisabled => write!(f, "{}", 255),
//         }
//     }
// }
