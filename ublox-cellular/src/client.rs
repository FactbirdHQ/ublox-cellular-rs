use atat::AtatClient;
use core::cell::{Cell, RefCell};
use embedded_hal::digital::OutputPin;
use heapless::{consts, ArrayLength, Bucket, FnvIndexMap, IndexMap, Pos, PowerOfTwo, String};

use crate::{
    command::{
        control::{types::*, *},
        gpio::{types::*, *},
        mobile_control::{types::*, *},
        system_features::{types::*, *},
        Urc, *,
    },
    error::Error,
    gprs::APNInfo,
    socket::{SocketHandle, SocketSet, SocketType, TcpSocket, UdpSocket},
};
use general::GetCCID;

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub enum State {
    Deregistered,
    Registering,
    Registered,
    Detached,
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
    pub(crate) apn_info: APNInfo,
    pin: String<consts::U4>,
}

impl<RST, DTR> Config<RST, DTR>
where
    RST: OutputPin,
    DTR: OutputPin,
{
    pub fn new(apn_info: APNInfo, pin: &str) -> Self {
        Config {
            rst_pin: None,
            dtr_pin: None,
            baud_rate: 115_200_u32,
            low_power_mode: false,
            flow_control: false,
            apn_info,
            pin: String::from(pin),
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

    pub fn pin(&self) -> &str {
        &self.pin
    }
}

pub struct GsmClient<C, RST, DTR, N, L>
where
    C: AtatClient,
    N: 'static
        + ArrayLength<Option<crate::sockets::SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>,
    L: 'static + ArrayLength<u8>,
{
    initialized: Cell<bool>,
    pub(crate) config: Config<RST, DTR>,
    pub(crate) state: Cell<State>,
    pub(crate) data_available: RefCell<FnvIndexMap<u8, usize, N>>,
    pub(crate) client: RefCell<C>,
    // Ublox devices can hold a maximum of 6 active sockets
    pub(crate) sockets: RefCell<&'static mut SocketSet<N, L>>,
}

impl<C, RST, DTR, N, L> GsmClient<C, RST, DTR, N, L>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>
        + ArrayLength<Bucket<u8, usize>>
        + ArrayLength<Option<Pos>>
        + PowerOfTwo,
    L: ArrayLength<u8>,
{
    pub fn new(
        client: C,
        socket_set: &'static mut SocketSet<N, L>,
        config: Config<RST, DTR>,
    ) -> Self {
        GsmClient {
            config,
            state: Cell::new(State::Deregistered),
            data_available: RefCell::new(IndexMap::new()),
            initialized: Cell::new(false),
            client: RefCell::new(client),
            sockets: RefCell::new(socket_set),
        }
    }

    /// Initilize a new ublox device to a known state (restart, wait for startup, set RS232 settings, gpio settings, etc.)
    pub fn init(&self, restart: bool) -> Result<(), Error> {
        defmt::info!("Initializing!");
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

        if self.initialized.get() {
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

        // defmt::info!("{:?}", self.send_internal(&GetIndicatorControl)?);
        // FIXME: defmt doesn't currently allow logging u128 types
        let ccid = self.send_internal(&GetCCID, false)?.ccid;
        defmt::info!("{:?}", ccid.to_le_bytes());

        self.initialized.set(true);

        Ok(())
    }

    #[inline]
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

    #[inline]
    fn autosense(&self) -> Result<(), Error> {
        for _ in 0..15 {
            match self.client.try_borrow_mut()?.send(&AT) {
                Ok(_) => {
                    return Ok(());
                }
                Err(_e) => {}
            };
        }
        Err(Error::BaudDetection)
    }

    #[inline]
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
        self.handle_urc().ok();

        for (socket, available) in self
            .data_available
            .try_borrow_mut()?
            .iter_mut()
            .filter(|(_, l)| **l > 0)
        {
            match self.socket_ingress(SocketHandle(*socket), *available) {
                Ok(bytes) => {
                    *available = *available - bytes;
                    defmt::trace!(
                        "[Socket({:u8})] Ingressed {:usize} bytes, {:usize} remaining",
                        socket,
                        bytes,
                        available
                    );
                    Ok(())
                }
                Err(Error::BufferFull) => {
                    defmt::warn!("[Socket({:u8})] Ingress buffer full", socket);
                    Ok(())
                }
                Err(e) => {
                    defmt::error!("[Socket({:u8})] Failed ingress! {:?}", socket, e);
                    Err(e)
                }
            }?;
        }

        Ok(())
    }

    fn handle_urc(&self) -> Result<(), Error> {
        let urc = self.client.try_borrow_mut()?.check_urc::<Urc>();

        match urc {
            Some(Urc::MessageWaitingIndication(_)) => {
                defmt::info!("[URC] MessageWaitingIndication");
                Ok(())
            }
            Some(Urc::SocketClosed(ip_transport_layer::urc::SocketClosed { socket })) => {
                defmt::info!("[URC] SocketClosed {:u8}", socket.0);
                let mut sockets = self.sockets.try_borrow_mut()?;
                match sockets.socket_type(socket) {
                    Some(SocketType::Tcp) => {
                        let mut tcp = sockets.get::<TcpSocket<_>>(socket)?;
                        tcp.close();
                    }
                    Some(SocketType::Udp) => {
                        let mut udp = sockets.get::<UdpSocket<_>>(socket)?;
                        udp.close();
                    }
                    _ => {}
                }
                sockets.remove(socket)?;
                Ok(())
            }
            Some(Urc::DataConnectionActivated(psn::urc::DataConnectionActivated { result })) => {
                defmt::info!("[URC] DataConnectionActivated {:u8}", result);
                Ok(())
            }
            Some(Urc::DataConnectionDeactivated(psn::urc::DataConnectionDeactivated {
                profile_id,
            })) => {
                defmt::info!("[URC] DataConnectionDeactivated {:u8}", profile_id);
                self.init(false)?;
                self.state.set(State::Deregistered);
                Ok(())
            }
            Some(Urc::SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable {
                socket,
                length,
            })) => {
                defmt::trace!(
                    "[Socket({:u8})] {:u16} bytes available",
                    socket.0,
                    length as u16
                );

                self.data_available
                    .try_borrow_mut()?
                    .insert(socket.0 as u8, length)
                    .map(drop)
                    .map_err(|_| Error::SocketNotFound)
            }
            None => Ok(()),
        }
    }

    #[inline]
    pub(crate) fn send_internal<A: atat::AtatCmd>(
        &self,
        req: &A,
        check_urc: bool,
    ) -> Result<A::Response, Error> {
        if check_urc {
            if let Err(e) = self.handle_urc() {
                defmt::error!("Failed handle URC: {:?}", e);
            }
        }

        self.client
            .try_borrow_mut()?
            .send(req)
            .map_err(|e| match e {
                nb::Error::Other(ate) => {
                    match core::str::from_utf8(&req.as_bytes()) {
                        Ok(s) => defmt::error!("{:?}: [{:str}]", ate, s),
                        Err(_) => defmt::error!(
                            "{:?}:",
                            ate,
                            // core::convert::AsRef::<[u8]>::as_ref(&req.as_bytes())
                        ),
                    };
                    ate.into()
                }
                nb::Error::WouldBlock => Error::_Unknown,
            })
    }

    pub fn send_at<A: atat::AtatCmd>(&self, cmd: &A) -> Result<A::Response, Error> {
        if !self.initialized.get() {
            self.init(false)?;
        }

        self.send_internal(cmd, true)
    }
}
