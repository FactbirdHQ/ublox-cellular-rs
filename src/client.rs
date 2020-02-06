use crate::{command::*, ATClient};

use at::ATInterface;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::timer::CountDown;
use log::error;

#[macro_export]
macro_rules! wait_for_unsolicited {
    ($client:expr, $p:pat) => {{
        let mut res: nb::Result<UnsolicitedResponse, at::Error> = Err(nb::Error::WouldBlock);
        if let Ok(ResponseType::Unsolicited(_)) = $client.client.peek_response() {
            res = match $client.client.wait_response() {
                Ok(ResponseType::Unsolicited(r)) => {
                    info!("{:?}", r);
                    if let $p = r {
                        Ok(r)
                    } else {
                        Err(nb::Error::WouldBlock)
                    }
                }
                Err(e) => Err(nb::Error::Other(e)),
                _ => Err(nb::Error::WouldBlock),
            }
        }
        res
    }};
}

pub struct GSMClient<T, RST, DTR>
where
    T: CountDown,
{
    initialized: bool,
    low_power_mode: bool,
    rst_pin: Option<RST>,
    dtr_pin: Option<DTR>,
    pub(crate) client: ATClient<T>,
}

impl<T, U, RST, DTR> GSMClient<T, RST, DTR>
where
    T: CountDown<Time = U>,
    U: From<u32>,
    T::Time: Copy,
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new(client: ATClient<T>, rst_pin: Option<RST>, dtr_pin: Option<DTR>) -> Self {
        GSMClient {
            initialized: false,
            low_power_mode: false,
            client,
            rst_pin,
            dtr_pin,
        }
    }

    pub fn init(&mut self, restart: bool) -> Result<(), at::Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
        // size_of!(Command);
        // size_of!(Response);
        // size_of!(ResponseType);

        if restart && self.rst_pin.is_some() {
            if let Some(ref mut rst) = self.rst_pin {
                rst.set_high().ok();
                // delay(1000);
                rst.set_low().ok();
            }
        } else {
            self.autosense()?;

            self.reset()?;
        }

        self.autosense()?;

        // Configure baud rate and flow control
        self.send_internal(RequestType::Cmd(Command::Flow))?;
        self.send_internal(RequestType::Cmd(Command::Baud))?;

        // Setup BaudRate, FlowControl, S3, S4, Echo

        // if self.dtr_pin.is_some() {
        //     self.low_power_mode(false)?;

        //     self.send_internal(RequestType::Cmd(Command::SetPowerSavingControl {
        //         mode: PowerSavingMode::CtrlByDtr,
        //         timeout: None,
        //     }))?;
        // }

        self.send_internal(RequestType::Cmd(Command::SetReportMobileTerminationError {
            n: TerminationErrorMode::Verbose,
        }))?;
        self.send_internal(RequestType::Cmd(Command::SetGpioConfiguration {
            gpio_id: 16,
            gpio_mode: GpioMode::GsmTxIndication,
        }))?;
        self.send_internal(RequestType::Cmd(Command::SetGpioConfiguration {
            gpio_id: 23,
            gpio_mode: GpioMode::NetworkStatus,
        }))?;

        self.send_internal(RequestType::Cmd(Command::GetCCID))?;
        self.send_internal(RequestType::Cmd(Command::GetIMEI))?;

        self.initialized = true;
        Ok(())
    }

    fn low_power_mode(&mut self, enable: bool) -> Result<(), at::Error> {
        if let Some(ref mut dtr) = self.dtr_pin {
            self.low_power_mode = enable;

            if enable {
                dtr.set_high().ok();
            } else {
                dtr.set_low().ok();
            }
            return Ok(());
        }
        Ok(())
    }

    fn autosense(&mut self) -> Result<(), at::Error> {
        block!(self.client.send(RequestType::Cmd(Command::AT))).map_err(|e| at::Error::Write)?;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), at::Error> {
        // block!(self.client.send(RequestType::Cmd(Command::SetModuleFunctionality {
        //     fun: Functionality::SilentResetWithSimReset,
        //     rst: None
        // }))).map_err(|e| at::Error::Write)?;
        Ok(())
    }

    fn send_internal(&mut self, req: RequestType) -> Result<ResponseType, at::Error> {
        // Should this automatically transition between SerialModes,
        // or return error on wrong RequestType for current SerialMode?
        block!(self.client.send(req.clone())).map_err(|e| {
            error!("{:?}\r", e);
            at::Error::Write
        })
    }

    pub fn send_at(&mut self, cmd: Command) -> Result<ResponseType, at::Error> {
        if !self.initialized {
            self.init(false)?
        }

        self.send_internal(RequestType::Cmd(cmd))
    }
}
