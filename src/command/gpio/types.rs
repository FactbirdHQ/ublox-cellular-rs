//! Argument and parameter types used by GPIO Commands and Responses

use atat::atat_derive::AtatEnum;

/// GPIO output value (for output function <`gpio_mode>=0` only):
#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum GpioOutValue {
    Low = 0,
    High = 1,
}

/// GPIO input value (for input function <`gpio_mode>=1` only):
#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum GpioInPull {
    /// (default value): no resistor activated
    NoPull = 0,
    /// pull up resistor active
    PullUp = 1,
    /// pull down resistor active
    PullDown = 2,
}

#[derive(Clone, PartialEq, Eq, AtatEnum)]
pub enum GpioMode {
    /// • 0: output
    #[at_arg(value = 0)]
    Output(GpioOutValue),
    /// • 1: input
    #[at_arg(value = 1)]
    Input(GpioInPull),
    /// • 2: network status indication
    #[at_arg(value = 2)]
    NetworkStatus,
    /// • 3: external GNSS supply enable
    #[at_arg(value = 3)]
    ExternalGnssSupplyEnable,
    /// • 4: external GNSS data ready
    #[at_arg(value = 4)]
    ExternalGnssDataReady,
    /// • 5: external GNSS RTC sharing
    #[at_arg(value = 5)]
    ExternalGnssRtcSharing,
    /// • 6: jamming detection indication
    #[at_arg(value = 6)]
    JammingDetection,
    /// • 7: SIM card detection
    #[at_arg(value = 7)]
    SimDetection,
    /// • 8: headset detection
    #[at_arg(value = 8)]
    HeadsetDetection,
    /// • 9: GSM Tx burst indication
    #[at_arg(value = 9)]
    GsmTxIndication,
    /// • 10: module operating status indication
    #[at_arg(value = 10)]
    ModuleOperatingStatus,
    /// • 11: module functionality status indication
    #[at_arg(value = 11)]
    ModuleFunctionalityStatus,
    /// • 12: I2S digital audio interface
    #[at_arg(value = 12)]
    I2SDigitalAudio,
    /// • 13: SPI serial interface
    #[at_arg(value = 13)]
    SpiSerial,
    /// • 14: master clock generation
    #[at_arg(value = 14)]
    MasterClockGeneration,
    /// • 15: UART (DSR, DTR, DCD e RI) interface
    #[at_arg(value = 15)]
    Uart,
    /// • 16: Wi-Fi enable
    #[at_arg(value = 16)]
    WifiEnable,
    /// • 18: ring indication
    #[at_arg(value = 18)]
    RingIndication,
    /// • 19: last gasp enable
    #[at_arg(value = 19)]
    LastGaspEnable,
    /// • 20: external GNSS antenna / LNA control enable
    #[at_arg(value = 20)]
    ExternalGnssAntenna,
    /// • 21: time pulse GNSS
    #[at_arg(value = 21)]
    TimePulseGnss,
    /// • 22: time pulse modem
    #[at_arg(value = 22)]
    TimePulseModem,
    /// • 23: time stamp of external interrupt
    #[at_arg(value = 23)]
    TimestampExternalInterrupt,
    /// • 24: fast power-off
    #[at_arg(value = 24)]
    FastPoweroff,
    /// • 25: LwM2M pulse
    #[at_arg(value = 25)]
    Lwm2mPulse,
    /// • 26: hardware flow control (RTS, CTS)
    #[at_arg(value = 26)]
    HardwareFlowControl,
    /// • 32: 32.768 kHz output
    #[at_arg(value = 32)]
    ClockOutput,
    /// • 255: pad disabled
    #[at_arg(value = 255)]
    PadDisabled,
}
