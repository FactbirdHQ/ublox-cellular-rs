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
    socket::{SocketHandle, SocketSet, SocketType, TcpSocket, UdpSocket},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Deregistered,
    Registering,
    Registered,
    Deattached,
    Attaching,
    Attached,
    Sending,
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
        // FIXME: Validate baudrates

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

pub struct GsmClient<C, RST, DTR>
where
    C: AtatClient,
{
    initialized: RefCell<bool>,
    config: Config<RST, DTR>,
    pub(crate) state: RefCell<State>,
    pub(crate) poll_cnt: RefCell<u8>,
    pub(crate) client: RefCell<C>,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: RefCell<SocketSet<consts::U2, consts::U2048>>,
}

impl<C, RST, DTR> GsmClient<C, RST, DTR>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new(client: C, config: Config<RST, DTR>) -> Self {
        GsmClient {
            config,
            state: RefCell::new(State::Deregistered),
            poll_cnt: RefCell::new(0),
            initialized: RefCell::new(false),
            client: RefCell::new(client),
            sockets: RefCell::new(SocketSet::new()),
        }
    }

    pub(crate) fn set_state(&self, state: State) -> Result<State, Error> {
        let prev_state = self.get_state()?;
        *self.state.try_borrow_mut().map_err(|_| Error::SetState)? = state;
        Ok(prev_state)
    }

    pub(crate) fn get_state(&self) -> Result<State, Error> {
        Ok(*self.state.try_borrow().map_err(|_| Error::SetState)?)
    }

    /// Initilize a new ublox device to a known state (restart, wait for startup, set RS232 settings, gpio settings, etc.)
    pub fn init(&self, restart: bool) -> Result<(), Error> {
        if restart && self.config.rst_pin.is_some() {
            if let Some(ref _rst) = self.config.rst_pin {
                // rst.set_high().ok();
                // delay(1000);
                // rst.set_low().ok();
            }
        } else {
            self.autosense()?;

            self.reset()?;
        }

        self.autosense()?;

        if *self.initialized.try_borrow()? {
            return Ok(());
        }

        if self.config.baud_rate > 230_400_u32 {
            // Needs a way to reconfigure uart baud rate temporarily
            // Relevant issue: https://github.com/rust-embedded/embedded-hal/issues/79
            return Err(Error::_Unknown);

            // self.send_internal(
            //     &SetDataRate {
            //         rate: BaudRate::B115200,
            //     },
            //     true,
            // )?;

            // NOTE: On the UART AT interface, after the reception of the "OK" result code for the +IPR command, the DTE
            // shall wait for at least 100 ms before issuing a new AT command; this is to guarantee a proper baud rate
            // reconfiguration.

            // UART end
            // delay(100);
            // UART begin(self.config.baud_rate)

            // self.autosense()?;
        }

        if self.config.flow_control {
            self.send_internal(
                &SetFlowControl {
                    value: FlowControl::RtsCts,
                },
                false,
            )?;
        } else {
            self.send_internal(
                &SetFlowControl {
                    value: FlowControl::Disabled,
                },
                false,
            )?;
        }

        if self.config.dtr_pin.is_some() && self.config.low_power_mode {
            self.low_power_mode(self.config.low_power_mode)?;

            self.send_internal(
                &SetPowerSavingControl {
                    mode: PowerSavingMode::CtrlByDtr,
                    timeout: None,
                },
                false,
            )?;
        } else {
            self.send_internal(
                &SetPowerSavingControl {
                    mode: PowerSavingMode::Disabled,
                    timeout: None,
                },
                false,
            )?;
        }

        self.send_internal(
            &SetReportMobileTerminationError {
                n: TerminationErrorMode::Disabled,
            },
            false,
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
            false,
        )?;
        self.send_internal(
            &SetGpioConfiguration {
                gpio_id: 16,
                gpio_mode: GpioMode::GsmTxIndication,
            },
            false,
        )?;
        self.send_internal(
            &SetGpioConfiguration {
                gpio_id: 23,
                gpio_mode: GpioMode::NetworkStatus,
            },
            false,
        )?;

        // info!("{:?}", self.send_internal(&GetIndicatorControl)?);
        // info!("{:?}", self.send_internal(&GetIMEI { snt: None })?);

        *self.initialized.try_borrow_mut()? = true;

        Ok(())
    }

    fn low_power_mode(&self, _enable: bool) -> Result<(), atat::Error> {
        if let Some(ref _dtr) = self.config.dtr_pin {
            // if enable {
            // dtr.set_high().ok();
            // } else {
            // dtr.set_low().ok();
            // }
            return Ok(());
        }
        Ok(())
    }

    fn autosense(&self) -> Result<(), Error> {
        for _ in 0..15 {
            match self.send_internal(&AT, false) {
                Ok(_) => {
                    return Ok(());
                }
                Err(_e) => {}
            };
        }
        Err(Error::BaudDetection)
    }

    fn reset(&self) -> Result<(), Error> {
        self.send_internal(
            &SetModuleFunctionality {
                fun: Functionality::SilentResetWithSimReset,
                rst: None,
            },
            false,
        )?;
        Ok(())
    }

    pub fn spin(&self) -> Result<(), Error> {
        self.handle_urc()?;

        // Occasionally poll every open socket, in case a `SocketDataAvailable`
        // URC was missed somehow
        let data_available: heapless::Vec<(SocketHandle, usize), consts::U6> = {
            let sockets = self.sockets.try_borrow()?;

            if sockets.len() > 0 && self.poll_cnt(false) >= 50 {
                self.poll_cnt(true);

                sockets
                    .iter()
                    .filter_map(|(h, s)| {
                        // Figure out if socket is TCP or UDP
                        match s.get_type() {
                            SocketType::Tcp => self
                                .send_internal(
                                    &ReadSocketData {
                                        socket: &h,
                                        length: 0,
                                    },
                                    false,
                                )
                                .map_or(None, |s| {
                                    if s.length > 0 {
                                        Some((h, s.length))
                                    } else {
                                        None
                                    }
                                }),
                            SocketType::Udp => self
                                .send_internal(
                                    &ReadUDPSocketData {
                                        socket: &h,
                                        length: 0,
                                    },
                                    false,
                                )
                                .map_or(None, |s| {
                                    if s.length > 0 {
                                        Some((h, s.length))
                                    } else {
                                        None
                                    }
                                }),
                            _ => None,
                        }
                    })
                    .collect()
            } else {
                heapless::Vec::new()
            }
        };

        data_available
            .iter()
            .try_for_each(|(handle, len)| self.socket_ingress(&handle, *len).map(|_| ()))
            .map_err(|e| {
                #[cfg(feature = "logging")]
                log::error!("ERROR: {:?}", e);
                e
            })?;

        Ok(())
    }

    fn handle_urc(&self) -> Result<(), Error> {
        let urc = self.client.try_borrow_mut()?.check_urc::<Urc>();

        match urc {
            Some(Urc::MessageWaitingIndication(_)) => {
                // #[cfg(feature = "logging")]
                // log::info!("[URC] MessageWaitingIndication");
                Ok(())
            }
            Some(Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket })) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] SocketClosed");
                let mut sockets = self.sockets.try_borrow_mut()?;
                match sockets.socket_type(&socket) {
                    Some(SocketType::Tcp) => {
                        let mut tcp = sockets.get::<TcpSocket<_>>(&socket)?;
                        tcp.close();
                    }
                    Some(SocketType::Udp) => {
                        let mut udp = sockets.get::<UdpSocket<_>>(&socket)?;
                        udp.close();
                    }
                    _ => {}
                }
                Ok(())
            }
            Some(Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                ..
            })) => {
                #[cfg(feature = "logging")]
                log::info!("[URC] DataConnectionDeactivated");
                Ok(self.set_state(State::Deattached).map(|_| ())?)
            }
            Some(Urc::SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable {
                socket,
                length,
            })) => match self.socket_ingress(&socket, length) {
                Ok(bytes) if bytes > 0 => {
                    // #[cfg(feature = "logging")]
                    // log::info!("[URC] Ingressed {:?} bytes", bytes);
                    Ok(())
                }
                Ok(_) => Ok(()),
                Err(e) => {
                    #[cfg(feature = "logging")]
                    log::error!("[URC] Failed ingress! {:?}", e);
                    Err(e.into())
                }
            },
            None => Ok(()),
        }
    }

    pub(crate) fn send_internal<A: atat::AtatCmd>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error> {
        if check_urc {
            if let Err(_e) = self.handle_urc() {
                #[cfg(features = "logging")]
                log::error!("Failed handle URC: {:?}", _e);
            }
        }

        self.client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    #[cfg(feature = "logging")]
                    match core::str::from_utf8(&req.as_bytes()) {
                        Ok(s) => log::error!("{:?}: [{:?}]", ate, s),
                        Err(_) => log::error!("{:?}: [{:?}]", ate, req.as_bytes()),
                    };
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        if !*self.initialized.try_borrow()? {
            self.init(false)?;
        }

        self.send_internal(cmd, true)
    }
}
