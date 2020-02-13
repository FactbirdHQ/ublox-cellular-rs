use crate::command::*;

use atat::{ATATCmd, ATATInterface};
use embedded_hal::digital::v2::OutputPin;
use log::info;

use crate::socket::SocketSet;
use heapless::consts;

pub struct GSMClient<C, RST, DTR>
where
    C: ATATInterface,
{
    initialized: bool,
    _low_power_mode: bool,
    rst_pin: Option<RST>,
    _dtr_pin: Option<DTR>,
    pub(crate) client: C,
}

impl<C, RST, DTR> GSMClient<C, RST, DTR>
where
    C: ATATInterface,
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new(client: C, rst_pin: Option<RST>, _dtr_pin: Option<DTR>) -> Self {
        GSMClient {
            initialized: false,
            _low_power_mode: false,
            client,
            rst_pin,
            _dtr_pin,
        }
    }

    pub fn init(&mut self, restart: bool) -> Result<(), atat::Error> {
        // Initilize a new ublox device to a known state (set RS232 settings, restart, wait for startup etc.)
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

        // self.send_internal(&cmd::GetIMEI)?;
        // self.send_internal(cmd::ATS)?;

        // Configure baud rate and flow control
        // self.send_internal(RequestType::Cmd(Command::Flow))?;
        // self.send_internal(RequestType::Cmd(Command::Baud))?;

        // Setup BaudRate, FlowControl, S3, S4, Echo

        // if self.dtr_pin.is_some() {
        //     self.low_power_mode(false)?;

        //     self.send_internal(RequestType::Cmd(Command::SetPowerSavingControl {
        //         mode: PowerSavingMode::CtrlByDtr,
        //         timeout: None,
        //     }))?;
        // }

        self.send_internal(&mobile_control::SetReportMobileTerminationError {
            n: mobile_control::types::TerminationErrorMode::Verbose,
        })?;
        self.send_internal(&gpio::SetGpioConfiguration {
            gpio_id: 16,
            gpio_mode: gpio::types::GpioMode::GsmTxIndication,
        })?;
        self.send_internal(&gpio::SetGpioConfiguration {
            gpio_id: 23,
            gpio_mode: gpio::types::GpioMode::NetworkStatus,
        })?;

        self.send_internal(&general::GetCCID {})?;
        let emei = self.send_internal(&general::GetIMEI { snt: None })?;
        info!("{:?}", emei);

        self.initialized = true;
        Ok(())
    }

    // fn low_power_mode(&mut self, enable: bool) -> Result<(), atat::Error> {
    //     if let Some(ref mut dtr) = self.dtr_pin {
    //         self.low_power_mode = enable;

    //         if enable {
    //             dtr.set_high().ok();
    //         } else {
    //             dtr.set_low().ok();
    //         }
    //         return Ok(());
    //     }
    //     Ok(())
    // }

    fn autosense(&mut self) -> Result<(), atat::Error> {
        // block!(self.client.send(RequestType::Cmd(Command::AT))).map_err(|e| atat::Error::Write)?;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), atat::Error> {
        // block!(self.client.send(RequestType::Cmd(Command::SetModuleFunctionality {
        //     fun: Functionality::SilentResetWithSimReset,
        //     rst: None
        // }))).map_err(|e| atat::Error::Write)?;
        Ok(())
    }

    pub fn poll(&mut self, sockets: &mut SocketSet<consts::U10>) -> Result<bool, ()> {
        let mut readiness_may_have_changed = false;
        loop {
            if self.socket_ingress(sockets)? {
                readiness_may_have_changed = true;
            } else {
                break;
            }
        }
        Ok(readiness_may_have_changed)
    }

    fn socket_ingress(&mut self, _sockets: &mut SocketSet<consts::U10>) -> Result<bool, ()> {
        let processed_any = false;
        // sockets.iter().filter_map(|socket| {
        //     self.send_at(Command::ReadSocketData {
        //         socket: socket.handle(),
        //         length: 256,
        //     })
        //     .map_err(|_e| ())?;
        //     Some(())
        // });
        Ok(processed_any)
    }

    fn send_internal<A: atat::ATATCmd>(&mut self, req: &A) -> Result<A::Response, atat::Error> {
        // Should this automatically transition between SerialModes,
        // or return error on wrong RequestType for current SerialMode?
        info!("Sending: [{}]\r", req.as_str());
        self.client.send(req).map_err(|e| atat::Error::Aborted)
    }

    pub fn send_at<A: atat::ATATCmd>(&mut self, cmd: &A) -> Result<A::Response, atat::Error> {
        if !self.initialized {
            self.init(false)?
        }

        self.send_internal(cmd)
    }
}
