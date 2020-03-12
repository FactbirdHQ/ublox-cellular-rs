use atat::prelude::*;
use core::cell::RefCell;
use embedded_hal::digital::v2::OutputPin;
use heapless::consts;

use crate::{
    command::{
        control::{types::*, *},
        general::*,
        gpio::{types::*, *},
        ip_transport_layer::{types::*, *},
        mobile_control::{types::*, *},
        network_service::*,
        system_features::{types::*, *},
        *,
    },
    error::Error,
    socket::{SocketHandle, SocketSet, TcpSocket},
};

pub enum State {
    Deattached,
    Attaching,
    Attached,
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Default)]
pub struct Config<RST, DTR> {
    rst_pin: Option<RST>,
    dtr_pin: Option<DTR>,
    baud_rate: u32,
    low_power_mode: bool,
    flow_control: bool,
}

impl<RST, DTR> Config<RST, DTR>
where
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new() -> Self {
        Config {
            rst_pin: None,
            dtr_pin: None,
            baud_rate: 115_200_u32,
            low_power_mode: false,
            flow_control: false,
        }
    }

    pub fn with_rst(self, rst_pin: RST) -> Self {
        Config {
            rst_pin: Some(rst_pin),
            ..self
        }
    }

    pub fn with_dtr(self, dtr_pin: DTR) -> Self {
        Config {
            dtr_pin: Some(dtr_pin),
            ..self
        }
    }

    pub fn baud_rate<B: Into<u32>>(self, baud_rate: B) -> Self {
        // TODO: Validate baudrates

        Config {
            baud_rate: baud_rate.into(),
            ..self
        }
    }

    pub fn with_flow_control(self) -> Self {
        Config {
            flow_control: true,
            ..self
        }
    }

    pub fn low_power_mode(self) -> Self {
        Config {
            low_power_mode: true,
            ..self
        }
    }
}

pub struct GSMClient<C, RST, DTR>
where
    C: ATATInterface,
{
    initialized: RefCell<bool>,
    config: Config<RST, DTR>,
    pub(crate) state: State,
    pub(crate) client: RefCell<C>,
    pub(crate) sockets: RefCell<SocketSet<consts::U10>>,
}

impl<C, RST, DTR> GSMClient<C, RST, DTR>
where
    C: ATATInterface,
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new(client: C, config: Config<RST, DTR>) -> Self {
        GSMClient {
            config,
            state: State::Deattached,
            initialized: RefCell::new(false),
            client: RefCell::new(client),
            sockets: RefCell::new(SocketSet::new()),
        }
    }

    /// Initilize a new ublox device to a known state (restart, wait for startup, set RS232 settings, gpio settings, etc.)
    pub fn init(&self, restart: bool) -> Result<(), Error> {
        if restart && self.config.rst_pin.is_some() {
            if let Some(ref rst) = self.config.rst_pin {
                // rst.set_high().ok();
                // delay(1000);
                // rst.set_low().ok();
            }
        } else {
            self.autosense()?;

            self.reset()?;
        }

        self.autosense()?;

        if self.config.baud_rate > 230_400_u32 {
            // Needs a way to reconfigure uart baud rate temporarily
            // Relevant issue: https://github.com/rust-embedded/embedded-hal/issues/79
            return Err(Error::_Unknown);

            self.send_internal(&SetDataRate {
                rate: BaudRate::B115200,
            })?;

            // NOTE: On the UART AT interface, after the reception of the "OK" result code for the +IPR command, the DTE
            // shall wait for at least 100 ms before issuing a new AT command; this is to guarantee a proper baud rate
            // reconfiguration.

            // UART end
            // delay(100);
            // UART begin(self.config.baud_rate)

            self.autosense()?;
        }

        if self.config.flow_control {
            self.send_internal(&SetFlowControl {
                value: FlowControl::RtsCts,
            })?;
        } else {
            self.send_internal(&SetFlowControl {
                value: FlowControl::Disabled,
            })?;
        }

        if self.config.dtr_pin.is_some() && self.config.low_power_mode {
            self.low_power_mode(self.config.low_power_mode)?;

            self.send_internal(&SetPowerSavingControl {
                mode: PowerSavingMode::CtrlByDtr,
                timeout: None,
            })?;
        } else {
            self.send_internal(&SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
            })?;
        }

        self.send_internal(&SetReportMobileTerminationError {
            n: TerminationErrorMode::Verbose,
        })?;

        self.send_internal(&SetGpioConfiguration {
            gpio_id: 42,
            gpio_mode: GpioMode::PadDisabled,
        })?;
        self.send_internal(&SetGpioConfiguration {
            gpio_id: 16,
            gpio_mode: GpioMode::GsmTxIndication,
        })?;
        self.send_internal(&SetGpioConfiguration {
            gpio_id: 23,
            gpio_mode: GpioMode::NetworkStatus,
        })?;

        // info!("{:?}\r", self.send_internal(&GetIndicatorControl)?);
        // info!("{:?}\r", self.send_internal(&GetIMEI { snt: None })?);

        self.initialized.swap(&RefCell::new(true));

        Ok(())
    }

    fn low_power_mode(&self, enable: bool) -> Result<(), atat::Error> {
        if let Some(ref dtr) = self.config.dtr_pin {
            if enable {
                // dtr.set_high().ok();
            } else {
                // dtr.set_low().ok();
            }
            return Ok(());
        }
        Ok(())
    }

    fn autosense(&self) -> Result<(), Error> {
        let mut attempt = 1;
        while attempt < 15 {
            match self.send_internal(&AT) {
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {}
            };
            attempt += 1;
        }
        Err(Error::BaudDetection)
    }

    fn reset(&self) -> Result<(), Error> {
        // self.send_internal(&SetModuleFunctionality {
        //     fun: Functionality::SilentResetWithSimReset,
        //     rst: None,
        // })?;
        Ok(())
    }

    pub fn handle_urc(&self) {
        let urc = if let Ok(ref mut client) = self.client.try_borrow_mut() {
            client.check_urc::<crate::command::Urc>()
        } else {
            None
        };

        match urc {
            Some(crate::command::Urc::MessageWaitingIndication(_)) => {}
            Some(crate::command::Urc::SocketDataAvailable(
                ip_transport_layer::urc::SocketDataAvailable { socket, length },
            )) => while self.socket_ingress(socket, length).is_err() {},
            Some(_) => log::info!("Some other URC\r"),
            _ => (),
        };
    }

    fn socket_ingress(&self, socket: SocketHandle, length: usize) -> Result<usize, ()> {
        let socket_data = self
            .send_at(&ReadSocketData { socket, length })
            .map_err(|e| ())?;

        let mut sockets = self.sockets.try_borrow_mut().map_err(|e| ())?;
        let mut tcp = sockets
            .get::<TcpSocket>(socket_data.socket)
            .map_err(|e| ())?;

        Ok(tcp.rx_enqueue_slice(&socket_data.data.as_bytes()))
    }

    fn send_internal<A: atat::ATATCmd>(&self, req: &A) -> Result<A::Response, Error> {
        self.client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    log::error!("{:?}: [{:?}]\r", ate, req.as_str());
                    ate.into()
                },
                _ => atat::Error::ResponseError.into(),
            })
    }

    pub fn send_at<A: atat::ATATCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        // if !self.initialized.try_borrow()? {
        //     self.init(false)?
        // }

        self.send_internal(cmd)
    }
}
