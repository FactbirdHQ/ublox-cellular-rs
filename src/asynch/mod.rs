pub mod control;
pub mod runner;
#[cfg(feature = "internal-network-stack")]
pub mod ublox_stack;

pub mod state;

use core::mem::MaybeUninit;

use crate::{
    command::{
        control::{types::FlowControl, SetFlowControl},
        ipc::SetMultiplexing,
        mobile_control::{
            types::{Functionality, ResetMode, TerminationErrorMode},
            SetModuleFunctionality, SetReportMobileTerminationError,
        },
        psn::{DeactivatePDPContext, EnterPPP, SetPDPContextDefinition},
        Urc,
    },
    config::{Apn, CellularConfig},
};
use atat::{
    asynch::{AtatClient, Client, SimpleClient},
    AtatIngress, UrcChannel,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};
use embedded_io::Error;
use embedded_io_async::{BufRead, Read, Write};
use runner::Runner;

use self::control::Control;

pub struct AtHandle<'d, AT: AtatClient>(&'d Mutex<NoopRawMutex, AT>);

impl<'d, AT: AtatClient> AtHandle<'d, AT> {
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
        self.0.lock().await.send_retry::<Cmd>(cmd).await
    }
}

#[cfg(feature = "ppp")]
pub type Resources<
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> = UbxResources<
    embassy_at_cmux::ChannelTx<'static, 256>,
    CMD_BUF_SIZE,
    INGRESS_BUF_SIZE,
    URC_CAPACITY,
>;

#[cfg(feature = "internal-network-stack")]
pub use self::UbxResources as Resources;

pub struct UbxResources<
    W: Write,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    ch: state::State,

    res_slot: atat::ResponseSlot<INGRESS_BUF_SIZE>,
    urc_channel: UrcChannel<Urc, URC_CAPACITY, 2>,
    cmd_buf: [u8; CMD_BUF_SIZE],
    ingress_buf: [u8; INGRESS_BUF_SIZE],

    at_client: MaybeUninit<Mutex<NoopRawMutex, Client<'static, W, INGRESS_BUF_SIZE>>>,

    #[cfg(feature = "ppp")]
    ppp_state: embassy_net_ppp::State<2, 2>,

    #[cfg(feature = "ppp")]
    mux: embassy_at_cmux::Mux<2, 256>,
}

impl<
        W: Write,
        const CMD_BUF_SIZE: usize,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
    > UbxResources<W, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub fn new() -> Self {
        Self {
            ch: state::State::new(),

            res_slot: atat::ResponseSlot::new(),
            urc_channel: atat::UrcChannel::new(),
            cmd_buf: [0; CMD_BUF_SIZE],
            ingress_buf: [0; INGRESS_BUF_SIZE],

            at_client: MaybeUninit::uninit(),

            #[cfg(feature = "ppp")]
            ppp_state: embassy_net_ppp::State::new(),

            #[cfg(feature = "ppp")]
            mux: embassy_at_cmux::Mux::new(),
        }
    }
}

#[cfg(feature = "internal-network-stack")]
pub fn new_internal<
    'a,
    R: embedded_io_async::Read,
    W: embedded_io_async::Write,
    C: CellularConfig<'a>,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
>(
    reader: R,
    writer: W,
    resources: &'a mut Resources<W, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
    config: C,
) -> (
    state::Device<'a, Client<'a, W, INGRESS_BUF_SIZE>, URC_CAPACITY>,
    Control<'a, Client<'a, W, INGRESS_BUF_SIZE>>,
    InternalRunner<'a, R, W, C, INGRESS_BUF_SIZE, URC_CAPACITY>,
) {
    // safety: this is a self-referential struct, however:
    // - it can't move while the `'a` borrow is active.
    // - when the borrow ends, the dangling references inside the MaybeUninit will never be used again.
    let at_client_uninit: *mut MaybeUninit<Mutex<NoopRawMutex, Client<'a, W, INGRESS_BUF_SIZE>>> =
        (&mut resources.at_client
            as *mut MaybeUninit<Mutex<NoopRawMutex, Client<'static, W, INGRESS_BUF_SIZE>>>)
            .cast();

    unsafe { &mut *at_client_uninit }.write(Mutex::new(Client::new(
        writer,
        &resources.res_slot,
        &mut resources.cmd_buf,
        atat::Config::default(),
    )));

    let at_client = unsafe { (&*at_client_uninit).assume_init_ref() };

    let (ch_runner, net_device) = state::new(
        &mut resources.ch,
        AtHandle(at_client),
        resources.urc_channel.subscribe().unwrap(),
    );

    let control = Control::new(ch_runner.state_runner(), AtHandle(at_client));

    let runner = Runner::new(
        ch_runner,
        AtHandle(at_client),
        config,
        resources.urc_channel.subscribe().unwrap(),
    );

    let ingress = atat::Ingress::new(
        atat::AtDigester::<Urc>::new(),
        &mut resources.ingress_buf,
        &resources.res_slot,
        &resources.urc_channel,
    );

    let runner = InternalRunner {
        cellular_runner: runner,
        ingress,
        reader,
    };

    (net_device, control, runner)
}

#[cfg(feature = "internal-network-stack")]
pub struct InternalRunner<
    'a,
    R: embedded_io_async::Read,
    W: embedded_io_async::Write,
    C: CellularConfig<'a>,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub cellular_runner: Runner<'a, Client<'a, W, INGRESS_BUF_SIZE>, C, URC_CAPACITY>,
    pub ingress: atat::Ingress<'a, atat::AtDigester<Urc>, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, 2>,
    pub reader: R,
}

#[cfg(feature = "internal-network-stack")]
impl<
        'a,
        R: embedded_io_async::Read,
        W: embedded_io_async::Write,
        C: CellularConfig<'a>,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
    > InternalRunner<'a, R, W, C, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub async fn run(&mut self) -> ! {
        embassy_futures::join::join(
            self.ingress.read_from(&mut self.reader),
            self.cellular_runner.run(),
        )
        .await;
        core::unreachable!()
    }
}

#[cfg(feature = "ppp")]
pub fn new_ppp<
    'a,
    C: CellularConfig<'a>,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
>(
    resources: &'a mut Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
    config: C,
) -> (
    embassy_net_ppp::Device<'a>,
    Control<'a, Client<'a, embassy_at_cmux::ChannelTx<'a, 256>, INGRESS_BUF_SIZE>>,
    PPPRunner<'a, C, INGRESS_BUF_SIZE, URC_CAPACITY>,
) {
    let ch_runner = state::new_ppp(&mut resources.ch);
    let state_ch = ch_runner.state_runner();

    let (mux_runner, [ppp_channel, control_channel]) = resources.mux.start();
    let (control_rx, control_tx, _) = control_channel.split();

    // safety: this is a self-referential struct, however:
    // - it can't move while the `'a` borrow is active.
    // - when the borrow ends, the dangling references inside the MaybeUninit will never be used again.
    let at_client_uninit: *mut MaybeUninit<
        Mutex<NoopRawMutex, Client<'a, embassy_at_cmux::ChannelTx<'a, 256>, INGRESS_BUF_SIZE>>,
    > = (&mut resources.at_client
        as *mut MaybeUninit<
            Mutex<
                NoopRawMutex,
                Client<'static, embassy_at_cmux::ChannelTx<'static, 256>, INGRESS_BUF_SIZE>,
            >,
        >)
        .cast();

    unsafe { &mut *at_client_uninit }.write(Mutex::new(Client::new(
        control_tx,
        &resources.res_slot,
        &mut resources.cmd_buf,
        atat::Config::default(),
    )));

    let at_client = unsafe { (&*at_client_uninit).assume_init_ref() };

    let cellular_runner = Runner::new(
        ch_runner,
        AtHandle(at_client),
        config,
        resources.urc_channel.subscribe().unwrap(),
    );

    let ingress = atat::Ingress::new(
        atat::AtDigester::<Urc>::new(),
        &mut resources.ingress_buf,
        &resources.res_slot,
        &resources.urc_channel,
    );

    let control = Control::new(state_ch, AtHandle(at_client));

    let (net_device, ppp_runner) = embassy_net_ppp::new(&mut resources.ppp_state);

    let runner = PPPRunner {
        ppp_runner,
        cellular_runner,
        ingress,
        ppp_channel,
        control_rx,
        mux_runner,
    };

    (net_device, control, runner)
}

pub struct ReadWriteAdapter<R, W>(pub R, pub W);

impl<R, W> embedded_io_async::ErrorType for ReadWriteAdapter<R, W> {
    type Error = embedded_io::ErrorKind;
}

impl<R: Read, W> Read for ReadWriteAdapter<R, W> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf).await.map_err(|e| e.kind())
    }
}

impl<R, W: Write> Write for ReadWriteAdapter<R, W> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.1.write(buf).await.map_err(|e| e.kind())
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.1.flush().await.map_err(|e| e.kind())
    }
}

#[cfg(feature = "ppp")]
pub struct PPPRunner<
    'a,
    C: CellularConfig<'a>,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub ppp_runner: embassy_net_ppp::Runner<'a>,
    pub cellular_runner: Runner<
        'a,
        Client<'a, embassy_at_cmux::ChannelTx<'a, 256>, INGRESS_BUF_SIZE>,
        C,
        URC_CAPACITY,
    >,
    pub ingress: atat::Ingress<'a, atat::AtDigester<Urc>, Urc, INGRESS_BUF_SIZE, URC_CAPACITY, 2>,
    pub ppp_channel: embassy_at_cmux::Channel<'a, 256>,
    pub control_rx: embassy_at_cmux::ChannelRx<'a, 256>,
    pub mux_runner: embassy_at_cmux::Runner<'a, 2, 256>,
}

#[cfg(feature = "ppp")]
impl<'a, C: CellularConfig<'a>, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    PPPRunner<'a, C, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    async fn configure_apn<A: AtatClient>(at_client: &mut A) -> Result<(), atat::Error> {
        at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Minimum,
                rst: Some(ResetMode::DontReset),
            })
            .await?;

        let apn = match C::APN {
            Apn::None => "",
            Apn::Given { name, .. } => name,
        };

        at_client
            .send(&SetPDPContextDefinition {
                cid: C::CONTEXT_ID,
                pdp_type: "IP",
                apn,
            })
            .await?;

        at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Full,
                rst: Some(ResetMode::DontReset),
            })
            .await?;
        Ok(())
    }

    async fn init<R: Read, W: Write>(rx: &mut R, tx: &mut W) -> Result<(), atat::Error> {
        let mut buf = [0u8; 64];
        let mut at_client = SimpleClient::new(
            ReadWriteAdapter(rx, tx),
            atat::AtDigester::<Urc>::new(),
            &mut buf,
            atat::Config::default(),
        );

        at_client
            .send(&SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            })
            .await?;

        at_client
            .send(&SetFlowControl {
                value: FlowControl::RtsCts,
            })
            .await?;

        at_client
            .send(&SetMultiplexing {
                mode: 0,
                subset: None,
                port_speed: None,
                n1: None,
                t1: None,
                n2: None,
                t2: None,
                t3: None,
                k: None,
            })
            .await?;

        Ok(())
    }

    pub async fn run<R: BufRead + Read, W: Write>(
        &mut self,
        mut rx: R,
        mut tx: W,
        stack: &embassy_net::Stack<embassy_net_ppp::Device<'a>>,
    ) -> ! {
        loop {
            // Reset modem
            // if self.cellular_runner.init().await.is_err() {
            //     Timer::after(Duration::from_secs(5)).await;
            //     continue;
            // }

            // Timer::after(Duration::from_secs(5)).await;

            // Do AT init and enter CMUX mode using interface
            if Self::init(&mut rx, &mut tx).await.is_err() {
                Timer::after(Duration::from_secs(5)).await;
                continue;
            };

            Timer::after(Duration::from_secs(1)).await;

            let ppp_fut = async {
                let mut fails = 0;
                let mut last_start = None;

                loop {
                    Timer::after(Duration::from_secs(15)).await;

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

                    let mut buf = [0u8; 64];
                    let mut at_client = SimpleClient::new(
                        &mut self.ppp_channel,
                        atat::AtDigester::<Urc>::new(),
                        &mut buf,
                        atat::Config::default(),
                    );

                    if let Err(e) = Self::configure_apn(&mut at_client).await {
                        warn!("modem: configure failed {:?}", e);
                        continue;
                    }

                    Timer::after(Duration::from_secs(2)).await;

                    // hangup just in case a call was already in progress.
                    // Ignore errors because this fails if it wasn't.
                    let _ = at_client.send(&DeactivatePDPContext).await;

                    // Send AT command to enter PPP mode
                    let res = at_client.send(&EnterPPP { cid: C::CONTEXT_ID }).await;

                    if let Err(e) = res {
                        warn!("ppp dial failed {:?}", e);
                        continue;
                    }

                    drop(at_client);

                    // Check for CTS low (bit 2)
                    self.ppp_channel.set_hangup_detection(0x04, 0x00);

                    info!("RUNNING PPP");
                    let res = self
                        .ppp_runner
                        .run(&mut self.ppp_channel, C::PPP_CONFIG, |ipv4| {
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

                    self.ppp_channel.clear_hangup_detection();

                    // escape back to data mode.
                    self.ppp_channel.set_lines(0x44);
                    Timer::after(Duration::from_millis(100)).await;
                    self.ppp_channel.set_lines(0x46);
                }
            };

            let ingress_fut = async {
                self.ingress.read_from(&mut self.control_rx).await;
            };

            let mux_fut = async {
                self.mux_runner.run(&mut rx, &mut tx).await;
            };

            embassy_futures::select::select4(
                ppp_fut,
                ingress_fut,
                self.cellular_runner.run(),
                mux_fut,
            )
            .await;
        }
    }
}
