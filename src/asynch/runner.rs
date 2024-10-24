use core::{future::poll_fn, task::Poll};

use crate::{
    asynch::{network::NetDevice, state::OperationState},
    command::{
        control::{
            types::{
                BaudRate, Circuit108Behaviour, Circuit109Behaviour, Echo, ResultCodeSelection,
            },
            SetCircuit108Behaviour, SetCircuit109Behaviour, SetDataRate, SetEcho,
            SetResultCodeSelection,
        },
        device_lock::{responses::PinStatus, types::PinStatusCode, GetPinStatus},
        general::{responses::FirmwareVersion, GetCCID, GetFirmwareVersion, GetModelId},
        ipc::SetMultiplexing,
        mobile_control::{
            types::{Functionality, TerminationErrorMode},
            SetModuleFunctionality, SetReportMobileTerminationError,
        },
        network_service::SetChannelAndNetworkEnvDesc,
        networking::SetEmbeddedPortFiltering,
        psn::EnterPPP,
        system_features::{types::PowerSavingMode, SetPowerSavingControl},
        Urc, AT,
    },
    config::{CellularConfig, Transport},
    error::Error,
    modules::{Module, ModuleParams as _},
    DEFAULT_BAUD_RATE,
};

use super::{
    control::{Control, ProxyClient},
    pwr::PwrCtrl,
    state,
    urc_handler::UrcHandler,
    Resources,
};

use atat::{
    asynch::{AtatClient, SimpleClient},
    AtatIngress as _, UrcChannel,
};

use embassy_futures::{
    join::join,
    select::{select3, Either3},
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Channel};
use embassy_time::{with_timeout, Duration, Instant, Timer};
use embedded_io_async::Write as _;

pub(crate) const URC_SUBSCRIBERS: usize = 2;

pub(crate) const MAX_CMD_LEN: usize = 128;

pub const CMUX_MAX_FRAME_SIZE: usize = 128;
pub const CMUX_CHANNEL_SIZE: usize = CMUX_MAX_FRAME_SIZE * 4;

pub const CMUX_CHANNELS: usize = 2;

async fn at_bridge<'a, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>(
    (rx, tx): (
        &mut embassy_at_cmux::ChannelRx<'a, CMUX_CHANNEL_SIZE>,
        &mut embassy_at_cmux::ChannelTx<'a, CMUX_CHANNEL_SIZE>,
    ),
    req_slot: &Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
    ingress: &mut atat::Ingress<
        'a,
        atat::AtDigester<Urc>,
        Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
) -> ! {
    ingress.clear();

    let tx_fut = async {
        loop {
            let msg = req_slot.receive().await;
            let _ = tx.write_all(&msg).await;
        }
    };

    embassy_futures::join::join(tx_fut, ingress.read_from(rx)).await;

    unreachable!()
}

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'a, T, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    transport: T,

    pub ch: state::Runner<'a>,
    pub config: C,
    pub urc_channel: &'a UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,

    pub ingress: atat::Ingress<
        'a,
        atat::AtDigester<Urc>,
        Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
    pub res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    pub req_slot: &'a Channel<NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,

    pub mux_runner: embassy_at_cmux::Runner<'a, CMUX_CHANNELS, CMUX_CHANNEL_SIZE>,

    at_channel: (
        embassy_at_cmux::ChannelRx<'a, CMUX_CHANNEL_SIZE>,
        embassy_at_cmux::ChannelTx<'a, CMUX_CHANNEL_SIZE>,
        embassy_at_cmux::ChannelLines<'a, CMUX_CHANNEL_SIZE>,
    ),
    data_channel: embassy_at_cmux::Channel<'a, CMUX_CHANNEL_SIZE>,

    #[cfg(feature = "ppp")]
    pub ppp_runner: Option<embassy_net_ppp::Runner<'a>>,
}

impl<'a, T, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Runner<'a, T, C, INGRESS_BUF_SIZE, URC_CAPACITY>
where
    T: Transport,
    C: CellularConfig<'a> + 'a,
{
    pub fn new(
        transport: T,
        resources: &'a mut Resources<INGRESS_BUF_SIZE, URC_CAPACITY>,
        config: C,
    ) -> (Self, Control<'a, INGRESS_BUF_SIZE>) {
        let ch_runner = state::Runner::new(&mut resources.ch);

        let ingress = atat::Ingress::new(
            atat::AtDigester::<Urc>::new(),
            &mut resources.ingress_buf,
            &resources.res_slot,
            &resources.urc_channel,
        );

        let (mux_runner, channels) = resources.mux.start();
        let mut channel_iter = channels.into_iter();

        let at_channel = channel_iter.next().unwrap().split();
        let data_channel = channel_iter.next().unwrap();

        let control = Control::new(
            ch_runner.clone(),
            resources.req_slot.sender(),
            &resources.res_slot,
        );

        (
            Self {
                transport,

                ch: ch_runner,
                config,
                urc_channel: &resources.urc_channel,

                ingress,
                res_slot: &resources.res_slot,
                req_slot: &resources.req_slot,

                mux_runner,

                at_channel,
                data_channel,

                #[cfg(feature = "ppp")]
                ppp_runner: None,
            },
            control,
        )
    }

    #[cfg(feature = "ppp")]
    pub fn ppp_stack<'d: 'a, const N_RX: usize, const N_TX: usize>(
        &mut self,
        ppp_state: &'d mut embassy_net_ppp::State<N_RX, N_TX>,
    ) -> embassy_net_ppp::Device<'d> {
        let (net_device, ppp_runner) = embassy_net_ppp::new(ppp_state);
        self.ppp_runner.replace(ppp_runner);
        net_device
    }

    #[cfg(feature = "internal-network-stack")]
    pub fn internal_stack(&mut self) -> state::Device<URC_CAPACITY> {
        // let data_channel = self.data_channel;
        state::Device {
            shared: &self.ch.shared,
            desired_state_pub_sub: &self.ch.desired_state_pub_sub,
            urc_subscription: self.urc_channel.subscribe().unwrap(),
        }
    }

    /// Probe a given baudrate with the goal of establishing initial
    /// communication with the module, so we can reconfigure it for desired
    /// baudrate
    async fn probe_baud(&mut self, baudrate: BaudRate) -> Result<(), Error> {
        info!(
            "Probing cellular module using baud rate: {}",
            baudrate as u32
        );
        self.transport.set_baudrate(baudrate as u32);
        let mut cmd_buf = [0u8; 16];

        {
            let mut at_client = SimpleClient::new(
                &mut self.transport,
                atat::AtDigester::<Urc>::new(),
                &mut cmd_buf,
                C::AT_CONFIG,
            );

            // Allow auto bauding to kick in
            embassy_time::with_timeout(Duration::from_secs(5), async {
                loop {
                    if at_client.send(&AT).await.is_ok() {
                        break;
                    }
                    Timer::after(Duration::from_millis(100)).await;
                }
            })
            .await?;

            // Lets take a shortcut if we successfully probed for the desired
            // baudrate
            if baudrate == C::BAUD_RATE {
                return Ok(());
            }

            at_client
                .send_retry(&SetDataRate { rate: C::BAUD_RATE })
                .await?;
        }

        self.transport.set_baudrate(C::BAUD_RATE as u32);

        // On the UART AT interface, after the reception of the "OK" result code
        // for the +IPR command, the DTE shall wait for at least 100 ms before
        // issuing a new AT command; this is to guarantee a proper baud rate
        // reconfiguration
        Timer::after_millis(100).await;

        // Verify communication
        SimpleClient::new(
            &mut self.transport,
            atat::AtDigester::<Urc>::new(),
            &mut cmd_buf,
            C::AT_CONFIG,
        )
        .send_retry(&AT)
        .await?;

        Ok(())
    }

    async fn init(&mut self) -> Result<(), Error> {
        // Initialize a new ublox device to a known state (set RS232 settings)
        debug!("Initializing cellular module");

        let mut pwr = PwrCtrl::new(&self.ch, &mut self.config);
        if let Err(e) = pwr.power_up().await {
            pwr.power_down().await?;
            return Err(e);
        };

        // Probe all possible baudrates with the goal of establishing initial
        // communication with the module, so we can reconfigure it for desired
        // baudrate.
        //
        // Start with the two most likely
        let mut found_baudrate = false;

        for baudrate in [
            C::BAUD_RATE,
            DEFAULT_BAUD_RATE,
            BaudRate::B9600,
            BaudRate::B19200,
            BaudRate::B38400,
            BaudRate::B57600,
            BaudRate::B115200,
            BaudRate::B230400,
            BaudRate::B460800,
            BaudRate::B921600,
            BaudRate::B3000000,
        ] {
            match with_timeout(Duration::from_secs(6), self.probe_baud(baudrate)).await {
                Ok(Ok(_)) => {
                    if baudrate != C::BAUD_RATE {
                        // Attempt to store the desired baudrate, so we can shortcut
                        // this probing next time. Ignore any potential failures, as
                        // this is purely an optimization.

                        // TODO: Is this correct?
                        // Some modules seem to persist baud rates by themselves.
                        // Nothing to do here for now.
                    }
                    found_baudrate = true;
                    break;
                }
                _ => {}
            }
        }

        if !found_baudrate {
            // TODO: Attempt to do some better recovery here?
            PwrCtrl::new(&self.ch, &mut self.config)
                .power_down()
                .await?;

            return Err(Error::BaudDetection);
        }

        let mut cmd_buf = [0u8; 64];
        let mut at_client = SimpleClient::new(
            &mut self.transport,
            atat::AtDigester::<Urc>::new(),
            &mut cmd_buf,
            C::AT_CONFIG,
        );

        // FIXME:
        // // Tell module whether we support flow control
        // let flow_control = if C::FLOW_CONTROL {
        //     FlowControl::RtsCts
        // } else {
        //     FlowControl::Disabled
        // };

        // at_client
        //     .send_retry(&SetFlowControl {
        //         value: flow_control,
        //     })
        //     .await?;

        let model_id = at_client.send_retry(&GetModelId).await?;
        self.ch.set_module(Module::from_model_id(&model_id));

        let FirmwareVersion { version } = at_client.send_retry(&GetFirmwareVersion).await?;
        info!("Found module to be: {:?}, {:?}", self.ch.module(), version);

        at_client
            .send_retry(&SetEmbeddedPortFiltering {
                mode: C::EMBEDDED_PORT_FILTERING,
            })
            .await?;

        // Echo off
        at_client
            .send_retry(&SetEcho { enabled: Echo::Off })
            .await?;

        // Extended errors on
        at_client
            .send_retry(&SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            })
            .await?;

        #[cfg(feature = "internal-network-stack")]
        if C::HEX_MODE {
            at_client
                .send_retry(&crate::command::ip_transport_layer::SetHexMode {
                    hex_mode_disable: crate::command::ip_transport_layer::types::HexMode::Enabled,
                })
                .await?;
        } else {
            at_client
                .send_retry(&crate::command::ip_transport_layer::SetHexMode {
                    hex_mode_disable: crate::command::ip_transport_layer::types::HexMode::Disabled,
                })
                .await?;
        }

        // DCD circuit (109) changes in accordance with the carrier
        at_client
            .send_retry(&SetCircuit109Behaviour {
                value: Circuit109Behaviour::AlwaysPresent,
            })
            .await?;

        // Ignore changes to DTR
        at_client
            .send_retry(&SetCircuit108Behaviour {
                value: Circuit108Behaviour::Ignore,
            })
            .await?;

        // Check sim status
        let sim_status = async {
            for _ in 0..2 {
                if let Ok(PinStatus {
                    code: PinStatusCode::Ready,
                }) = at_client.send(&GetPinStatus).await
                {
                    debug!("SIM is ready");
                    return Ok(());
                }

                Timer::after_secs(1).await;
            }

            // There was an error initializing the SIM
            // We've seen issues on uBlox-based devices, as a precation, we'll cycle
            // the modem here through minimal/full functional state.
            at_client
                .send(&SetModuleFunctionality {
                    fun: self
                        .ch
                        .module()
                        .ok_or(Error::Uninitialized)?
                        .radio_off_cfun(),
                    rst: None,
                })
                .await?;
            at_client
                .send(&SetModuleFunctionality {
                    fun: Functionality::Full,
                    rst: None,
                })
                .await?;

            Err(Error::SimCard)
        };

        sim_status.await?;

        let ccid = at_client.send_retry(&GetCCID).await?;
        info!("CCID: {}", ccid.ccid);

        at_client
            .send_retry(&SetResultCodeSelection {
                value: ResultCodeSelection::ConnectOnly,
            })
            .await?;

        #[cfg(all(
            feature = "ucged",
            any(
                feature = "sara-r410m",
                feature = "sara-r412m",
                feature = "sara-r422",
                feature = "lara-r6"
            )
        ))]
        at_client
            .send_retry(&SetChannelAndNetworkEnvDesc {
                mode: if cfg!(feature = "ucged5") { 5 } else { 2 },
            })
            .await?;

        // Switch off UART power saving until it is integrated into this API
        at_client
            .send_retry(&SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
            })
            .await?;

        if self.ch.desired_state(None) == OperationState::Initialized {
            at_client
                .send_retry(&SetModuleFunctionality {
                    fun: self
                        .ch
                        .module()
                        .ok_or(Error::Uninitialized)?
                        .radio_off_cfun(),
                    rst: None,
                })
                .await?;
        }

        Ok(())
    }

    pub async fn run(&mut self, stack: embassy_net::Stack<'_>) -> ! {
        loop {
            let _ = PwrCtrl::new(&self.ch, &mut self.config).power_down().await;

            // Wait for the desired state to change to anything but `PowerDown`
            poll_fn(|cx| match self.ch.desired_state(Some(cx)) {
                OperationState::PowerDown => Poll::Pending,
                _ => Poll::Ready(()),
            })
            .await;

            if self.init().await.is_err() {
                continue;
            }

            #[cfg(feature = "ppp")]
            let ppp_fut = async {
                self.ch
                    .wait_for_operation_state(OperationState::DataEstablished)
                    .await;

                Timer::after_secs(1).await;

                let mut fails = 0;
                let mut last_start = None;

                loop {
                    if let Some(last_start) = last_start {
                        Timer::at(last_start + Duration::from_secs(10)).await;
                        // Do not attempt to start too fast.

                        // If was up stably for at least 1 min, reset fail counter.
                        if Instant::now() > last_start + Duration::from_secs(60) {
                            fails = 0;
                        } else {
                            fails += 1;
                            if fails == 10 {
                                warn!("modem: PPP failed too much, rebooting modem.");
                                break;
                            }
                        }
                    }
                    last_start = Some(Instant::now());

                    {
                        // Must be large enough to hold 'ATD*99***1#\r\n'
                        let mut buf = [0u8; 16];

                        let mut at_client = SimpleClient::new(
                            &mut self.data_channel,
                            atat::AtDigester::<Urc>::new(),
                            &mut buf,
                            C::AT_CONFIG,
                        );

                        // let _ = at_client.send(&DeactivatePDPContext).await;

                        // Send AT command to enter PPP mode
                        let res = at_client.send(&EnterPPP { cid: C::CONTEXT_ID }).await;

                        if let Err(e) = res {
                            warn!("ppp dial failed {:?}", e);
                            continue;
                        }

                        Timer::after(Duration::from_millis(100)).await;
                    }

                    // Check for CTS low (bit 2)
                    self.data_channel.set_hangup_detection(0x04, 0x00);

                    info!("RUNNING PPP");
                    let res = self
                        .ppp_runner
                        .as_mut()
                        .unwrap()
                        .run(&mut self.data_channel, C::PPP_CONFIG, |ipv4| {
                            debug!("Running on_ipv4_up for cellular!");

                            let Some(addr) = ipv4.address else {
                                warn!("PPP did not provide an IP address.");
                                return;
                            };
                            let mut dns_servers = heapless::Vec::new();
                            for s in ipv4.dns_servers.iter().flatten() {
                                let _ =
                                    dns_servers.push(embassy_net::Ipv4Address::from_bytes(&s.0));
                            }
                            let config =
                                embassy_net::ConfigV4::Static(embassy_net::StaticConfigV4 {
                                    address: embassy_net::Ipv4Cidr::new(
                                        embassy_net::Ipv4Address::from_bytes(&addr.0),
                                        0,
                                    ),
                                    gateway: None,
                                    dns_servers,
                                });

                            stack.set_config_v4(config);
                        })
                        .await;

                    info!("ppp failed: {:?}", res);

                    self.data_channel.clear_hangup_detection();

                    // escape back to data mode.
                    self.data_channel
                        .set_lines(embassy_at_cmux::Control::from_bits(0x44 << 1), None);
                    Timer::after(Duration::from_millis(100)).await;
                    self.data_channel
                        .set_lines(embassy_at_cmux::Control::from_bits(0x46 << 1), None);

                    if self.ch.desired_state(None) != OperationState::DataEstablished {
                        break;
                    }
                }
            };

            let mux_fut = async {
                // Must be large enough to hold 'AT+CMUX=0,0,5,512,10,3,40,10,2\r\n'
                let mut buf = [0u8; 32];
                {
                    let mut at_client = SimpleClient::new(
                        &mut self.transport,
                        atat::AtDigester::<Urc>::new(),
                        &mut buf,
                        C::AT_CONFIG,
                    );

                    at_client
                        .send(&SetMultiplexing {
                            mode: 0,
                            subset: Some(0),
                            port_speed: Some(5),
                            n1: Some(CMUX_MAX_FRAME_SIZE as u16),
                            t1: None, //Some(10),
                            n2: None, //Some(3),
                            t2: None, //Some(30),
                            t3: None, //Some(10),
                            k: None,  //Some(2),
                        })
                        .await
                        .unwrap();
                }

                // The UART interface takes around 200 ms to reconfigure itself
                // after the multiplexer configuration through the +CMUX AT
                // command.
                Timer::after_millis(200).await;

                // Drain the UART of any leftover AT stuff before setting up multiplexer
                let _ = embassy_time::with_timeout(Duration::from_millis(100), async {
                    loop {
                        let _ = self.transport.read(&mut buf).await;
                    }
                })
                .await;

                let (mut tx, mut rx) = self.transport.split_ref();

                // Signal to the rest of the driver, that the MUX is operational and running
                let fut = async {
                    // TODO: It should be possible to check on ChannelLines,
                    // when the MUX is opened successfully, instead of waiting a fixed time
                    Timer::after_secs(3).await;
                    self.ch.set_operation_state(OperationState::Initialized);
                };

                join(
                    self.mux_runner.run(&mut rx, &mut tx, CMUX_MAX_FRAME_SIZE),
                    fut,
                )
                .await
            };

            let device_fut = async {
                let (at_rx, at_tx, _) = &mut self.at_channel;

                let at_client = ProxyClient::new(self.req_slot.sender(), self.res_slot);
                let mut cell_device = NetDevice::<C, _>::new(&self.ch, &at_client);

                let mut urc_handler = UrcHandler::new(&self.ch, self.urc_channel);

                select3(
                    at_bridge((at_rx, at_tx), self.req_slot, &mut self.ingress),
                    urc_handler.run(),
                    cell_device.run(),
                )
                .await
            };

            #[cfg(feature = "ppp")]
            match select3(mux_fut, ppp_fut, device_fut).await {
                Either3::First(_) => {
                    warn!("Breaking to reboot modem from multiplexer");
                }
                Either3::Second(_) => {
                    warn!("Breaking to reboot modem from PPP");
                }
                Either3::Third(_) => {
                    warn!("Breaking to reboot modem from network runner");
                }
            }

            #[cfg(not(feature = "ppp"))]
            match embassy_futures::select::select(mux_fut, device_fut).await {
                embassy_futures::select::Either::First(_) => {
                    warn!("Breaking to reboot modem from multiplexer");
                }
                embassy_futures::select::Either::Second(_) => {
                    warn!("Breaking to reboot modem from network runner");
                }
            }
        }
    }
}
