use serde::{Serialize, Deserialize};
use ufmt::derive::uDebug;




#[derive(uDebug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(uDebug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Seconds(pub u32);