//! Argument and parameter types used by GPIO Commands and Responses

use atat::atat_derive::AtatEnum;

/// GPIO output value (for output function <gpio_mode>=0 only):
#[derive(Clone, PartialEq, AtatEnum)]
pub enum GpioOutValue {
    Low = 0,
    High = 1,
}

/// GPIO input value (for input function <gpio_mode>=1 only):
#[derive(Clone, PartialEq, AtatEnum)]
pub enum GpioInPull {
    /// (default value): no resistor activated
    NoPull = 0,
    /// pull up resistor active
    PullUp = 1,
    /// pull down resistor active
    PullDown = 2,
}

#[derive(Clone, PartialEq, AtatEnum)]
pub enum GpioMode {
    /// • 0: output
    // TODO: Correctly handle these in serde_at, see https://github.com/BlackbirdHQ/atat/issues/37
    // Output(GpioOutValue),
    Output = 0,
    /// • 1: input
    // Input(GpioInPull),
    Input = 1,
    /// • 2: network status indication
    NetworkStatus = 2,
    /// • 3: external GNSS supply enable
    ExternalGnssSupplyEnable = 3,
    /// • 4: external GNSS data ready
    ExternalGnssDataReady = 4,
    /// • 5: external GNSS RTC sharing
    ExternalGnssRtcSharing = 5,
    /// • 6: jamming detection indication
    JammingDetection = 6,
    /// • 7: SIM card detection
    SimDetection = 7,
    /// • 8: headset detection
    HeadsetDetection = 8,
    /// • 9: GSM Tx burst indication
    GsmTxIndication = 9,
    /// • 10: module operating status indication
    ModuleOperatingStatus = 10,
    /// • 11: module functionality status indication
    ModuleFunctionalityStatus = 11,
    /// • 12: I2S digital audio interface
    I2SDigitalAudio = 12,
    /// • 13: SPI serial interface
    SpiSerial = 13,
    /// • 14: master clock generation
    MasterClockGeneration = 14,
    /// • 15: UART (DSR, DTR, DCD e RI) interface
    Uart = 15,
    /// • 16: Wi-Fi enable
    WifiEnable = 16,
    /// • 18: ring indication
    RingIndication = 18,
    /// • 19: last gasp enable
    LastGaspEnable = 19,
    /// • 20: external GNSS antenna / LNA control enable
    ExternalGnssAntenna = 20,
    /// • 21: time pulse GNSS
    TimePulseGnss = 21,
    /// • 22: time pulse modem
    TimePulseModem = 22,
    /// • 23: time stamp of external interrupt
    TimestampExternalInterrupt = 23,
    /// • 24: fast power-off
    FastPoweroff = 24,
    /// • 25: LwM2M pulse
    Lwm2mPulse = 25,
    /// • 26: hardware flow control (RTS, CTS)
    HardwareFlowControl = 26,
    /// • 32: 32.768 kHz output
    ClockOutput = 32,
    /// • 255: pad disabled
    PadDisabled = 255,
}
