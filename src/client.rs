use atat::AtatClient;
use core::cell::RefCell;
use embedded_hal::digital::v2::OutputPin;
use heapless::consts;

use crate::{
    command::{
        control::{types::*, *},
        gpio::{types::*, *},
        ip_transport_layer::*,
        mobile_control::{types::*, *},
        system_features::{types::*, *},
        Urc, *,
    },
    error::Error,
    hex,
    socket::{SocketHandle, SocketSet, TcpSocket, UdpSocket},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Deregistered,
    Registering,
    Registered,
    Deattached,
    Attaching,
    Attached,
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
    C: AtatClient,
{
    initialized: RefCell<bool>,
    config: Config<RST, DTR>,
    pub(crate) state: RefCell<State>,
    pub(crate) client: RefCell<C>,
    pub(crate) sockets: RefCell<SocketSet<consts::U10>>,
}

impl<C, RST, DTR> GSMClient<C, RST, DTR>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new(client: C, config: Config<RST, DTR>) -> Self {
        GSMClient {
            config,
            state: RefCell::new(State::Deregistered),
            initialized: RefCell::new(false),
            client: RefCell::new(client),
            sockets: RefCell::new(SocketSet::new()),
        }
    }

    pub(crate) fn set_state(&self, state: State) -> Result<(), Error> {
        *self.state.try_borrow_mut().map_err(|_| Error::SetState)? = state;
        Ok(())
    }

    pub(crate) fn get_state(&self) -> Result<State, Error> {
        Ok(*self.state.try_borrow().map_err(|_| Error::SetState)?)
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

            self.send_internal(
                &SetDataRate {
                    rate: BaudRate::B115200,
                },
                true,
            )?;

            // NOTE: On the UART AT interface, after the reception of the "OK" result code for the +IPR command, the DTE
            // shall wait for at least 100 ms before issuing a new AT command; this is to guarantee a proper baud rate
            // reconfiguration.

            // UART end
            // delay(100);
            // UART begin(self.config.baud_rate)

            self.autosense()?;
        }

        if self.config.flow_control {
            self.send_internal(
                &SetFlowControl {
                    value: FlowControl::RtsCts,
                },
                true,
            )?;
        } else {
            self.send_internal(
                &SetFlowControl {
                    value: FlowControl::Disabled,
                },
                true,
            )?;
        }

        if self.config.dtr_pin.is_some() && self.config.low_power_mode {
            self.low_power_mode(self.config.low_power_mode)?;

            self.send_internal(
                &SetPowerSavingControl {
                    mode: PowerSavingMode::CtrlByDtr,
                    timeout: None,
                },
                true,
            )?;
        } else {
            self.send_internal(
                &SetPowerSavingControl {
                    mode: PowerSavingMode::Disabled,
                    timeout: None,
                },
                true,
            )?;
        }

        self.send_internal(
            &SetReportMobileTerminationError {
                n: TerminationErrorMode::Verbose,
            },
            true,
        )?;

        // self.send_internal(
        //     &general::IdentificationInformation {
        //         n: 9,
        //     },
        //     true,
        // )?;

        self.send_internal(
            &SetGpioConfiguration {
                gpio_id: 42,
                gpio_mode: GpioMode::PadDisabled,
            },
            true,
        )?;
        self.send_internal(
            &SetGpioConfiguration {
                gpio_id: 16,
                gpio_mode: GpioMode::GsmTxIndication,
            },
            true,
        )?;
        self.send_internal(
            &SetGpioConfiguration {
                gpio_id: 23,
                gpio_mode: GpioMode::NetworkStatus,
            },
            true,
        )?;

        // info!("{:?}", self.send_internal(&GetIndicatorControl)?);
        // info!("{:?}", self.send_internal(&GetIMEI { snt: None })?);

        *self.initialized.try_borrow_mut()? = true;

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
        for _ in 0..15 {
            match self.send_internal(&AT, true) {
                Ok(_) => {
                    return Ok(());
                }
                Err(_e) => {}
            };
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

    pub fn spin(&self) -> Result<(), Error> {
        self.handle_urcs()
    }

    fn handle_urcs(&self) -> Result<(), Error> {
        loop {
            let urc = self.client.try_borrow_mut()?.check_urc::<Urc>();

            match urc {
                Some(Urc::MessageWaitingIndication(_)) => {
                    log::info!("[URC] MessageWaitingIndication");
                }
                Some(Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket })) => {
                    let mut sockets = self.sockets.try_borrow_mut()?;
                    let mut tcp = sockets.get::<TcpSocket>(socket)?;
                    tcp.close();
                }
                Some(Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                    ..
                })) => {
                    self.set_state(State::Deattached)?;
                }
                Some(Urc::SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable {
                    socket,
                    length,
                })) => {
                    match self.socket_ingress(socket, length) {
                        Ok(_bytes) => {
                            // log::info!("[URC] Ingressed {:?} bytes", bytes)
                        }
                        Err(e) => log::error!("[URC] Failed ingress! {:?}", e),
                    }
                }
                None => break,
            };
        }
        Ok(())
    }

    fn socket_ingress(&self, socket: SocketHandle, length: usize) -> Result<usize, Error> {
        if length == 0 {
            return Ok(0);
        }
        let chunk_size = core::cmp::min(length, 200);
        let socket_data = self.send_at(&ReadSocketData {
            socket,
            length: chunk_size,
        })?;

        if socket_data.length != chunk_size {
            return Err(Error::BadLength);
        }

        // TODO: Handle this decoding in-place?
        let data: heapless::Vec<_, consts::U200> =
            hex::decode_hex(&socket_data.data).map_err(|_| Error::BadLength)?;

        let mut sockets = self.sockets.try_borrow_mut()?;
        match sockets.get::<TcpSocket>(socket_data.socket){
            Ok(mut tcp) => Ok(tcp.rx_enqueue_slice(&data)),
            Err(_) => {
                match sockets.get::<UdpSocket>(socket_data.socket){
                    Ok(mut udp) => Ok(udp.rx_enqueue_slice(&data)),
                    Err(e) => Err(Error::Socket(e))
                }
            }   
        }
    }

    pub(crate) fn send_internal<A: atat::AtatCmd>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error> {
        // React to any enqueued URC's before starting a new command exchange
        if check_urc {
            if let Err(e) = self.handle_urcs() {
                log::error!("Failed handle URC: {:?}", e);
            }
        }

        self.client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    log::error!("{:?}: [{:?}]", ate, req.as_string());
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        if !*self.initialized.try_borrow()? {
            self.init(false)?
        }

        self.send_internal(cmd, true)
    }
}
