use core::mem::MaybeUninit;

use crate::{
    command::{
        ipc::SetMultiplexing,
        psn::{DeactivatePDPContext, EnterPPP},
        Urc,
    },
    config::CellularConfig,
    module_timing::boot_time,
};
use atat::{
    asynch::{AtatClient, Client, SimpleClient},
    AtatIngress,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant, Timer};
use embedded_io_async::{BufRead, Error, ErrorKind, Read, Write};

use super::{
    control::Control,
    resources::UbxResources,
    runner::{Runner, URC_SUBSCRIBERS},
    state, AtHandle,
};

pub const CMUX_MAX_FRAME_SIZE: usize = 512;
pub const CMUX_CHANNEL_SIZE: usize = CMUX_MAX_FRAME_SIZE * 2;
pub const CMUX_CHANNELS: usize = 2; // AT Control + PPP data

pub type Resources<
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> = UbxResources<
    embassy_at_cmux::ChannelTx<'static, CMUX_CHANNEL_SIZE>,
    CMD_BUF_SIZE,
    INGRESS_BUF_SIZE,
    URC_CAPACITY,
>;

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
    Control<'a, Client<'a, embassy_at_cmux::ChannelTx<'a, CMUX_CHANNEL_SIZE>, INGRESS_BUF_SIZE>>,
    PPPRunner<'a, C, INGRESS_BUF_SIZE, URC_CAPACITY>,
) {
    let ch_runner = state::new_ppp(&mut resources.ch);
    let state_ch = ch_runner.state_runner();

    let (mux_runner, [control_channel, ppp_channel]) = resources.mux.start();
    let (control_rx, control_tx, _) = control_channel.split();

    // safety: this is a self-referential struct, however:
    // - it can't move while the `'a` borrow is active.
    // - when the borrow ends, the dangling references inside the MaybeUninit will never be used again.
    let at_client_uninit: *mut MaybeUninit<
        Mutex<
            NoopRawMutex,
            Client<'a, embassy_at_cmux::ChannelTx<'a, CMUX_CHANNEL_SIZE>, INGRESS_BUF_SIZE>,
        >,
    > = (&mut resources.at_client
        as *mut MaybeUninit<
            Mutex<
                NoopRawMutex,
                Client<
                    'static,
                    embassy_at_cmux::ChannelTx<'static, CMUX_CHANNEL_SIZE>,
                    INGRESS_BUF_SIZE,
                >,
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
        powered: false,
        ppp_runner,
        cellular_runner,
        ingress,
        ppp_channel,
        control_rx,
        mux_runner,
    };

    (net_device, control, runner)
}

pub struct PPPRunner<
    'a,
    C: CellularConfig<'a>,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub powered: bool,
    pub ppp_runner: embassy_net_ppp::Runner<'a>,
    pub cellular_runner: Runner<
        'a,
        Client<'a, embassy_at_cmux::ChannelTx<'a, CMUX_CHANNEL_SIZE>, INGRESS_BUF_SIZE>,
        C,
        URC_CAPACITY,
    >,
    pub ingress: atat::Ingress<
        'a,
        atat::AtDigester<Urc>,
        Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
    pub ppp_channel: embassy_at_cmux::Channel<'a, CMUX_CHANNEL_SIZE>,
    pub control_rx: embassy_at_cmux::ChannelRx<'a, CMUX_CHANNEL_SIZE>,
    pub mux_runner: embassy_at_cmux::Runner<'a, CMUX_CHANNELS, CMUX_CHANNEL_SIZE>,
}

impl<'a, C: CellularConfig<'a>, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    PPPRunner<'a, C, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    async fn init_multiplexer<R: Read, W: Write>(
        rx: &mut R,
        tx: &mut W,
    ) -> Result<(), crate::error::Error> {
        let mut buf = [0u8; 64];
        let mut interface = ReadWriteAdapter(rx, tx);
        let mut at_client = SimpleClient::new(
            &mut interface,
            atat::AtDigester::<Urc>::new(),
            &mut buf,
            atat::Config::default(),
        );

        super::runner::init_at(&mut at_client, C::FLOW_CONTROL).await?;

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
            .await?;

        drop(at_client);

        // Drain the UART of any leftover AT stuff before setting up multiplexer
        let _ = embassy_time::with_timeout(Duration::from_millis(100), async {
            loop {
                let _ = interface.read(&mut buf).await;
            }
        })
        .await;

        Ok(())
    }

    pub async fn run<R: BufRead + Read, W: Write>(
        &mut self,
        mut rx: R,
        mut tx: W,
        on_ipv4_up: impl FnMut(embassy_net_ppp::Ipv4Status) + Copy,
    ) -> ! {
        loop {
            if !self.powered {
                // Reset modem
                self.cellular_runner
                    .change_state_to_desired_state(state::OperationState::PowerDown)
                    .await;
                self.cellular_runner
                    .change_state_to_desired_state(state::OperationState::PowerUp)
                    .await;

                Timer::after(boot_time()).await;
            }

            // Do AT init and enter CMUX mode using interface
            if Self::init_multiplexer(&mut rx, &mut tx).await.is_err() {
                Timer::after(Duration::from_secs(5)).await;
                continue;
            };

            self.cellular_runner
                .change_state_to_desired_state(state::OperationState::DataEstablished)
                .await;

            let ppp_fut = async {
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
                        let mut buf = [0u8; 16]; // Enough room for "ATD*99***1#\r\n"
                        let mut at_client = SimpleClient::new(
                            &mut self.ppp_channel,
                            atat::AtDigester::<Urc>::new(),
                            &mut buf,
                            atat::Config::default(),
                        );

                        // hangup just in case a call was already in progress.
                        // Ignore errors because this fails if it wasn't.
                        let _ = at_client.send(&DeactivatePDPContext).await;

                        // Send AT command to enter PPP mode
                        let res = at_client.send(&EnterPPP { cid: C::CONTEXT_ID }).await;

                        if let Err(e) = res {
                            warn!("ppp dial failed {:?}", e);
                            continue;
                        }

                        Timer::after(Duration::from_millis(100)).await;
                    }

                    // Check for CTS low (bit 2)
                    // self.ppp_channel.set_hangup_detection(0x04, 0x00);

                    info!("RUNNING PPP");
                    let res = self
                        .ppp_runner
                        .run(&mut self.ppp_channel, C::PPP_CONFIG, on_ipv4_up)
                        .await;

                    info!("ppp failed: {:?}", res);

                    self.ppp_channel.clear_hangup_detection();

                    // escape back to data mode.
                    self.ppp_channel
                        .set_lines(embassy_at_cmux::Control::from_bits(0x44 << 1), None);
                    Timer::after(Duration::from_millis(100)).await;
                    self.ppp_channel
                        .set_lines(embassy_at_cmux::Control::from_bits(0x46 << 1), None);
                }
            };

            self.ingress.clear();

            embassy_futures::select::select4(
                self.mux_runner.run(&mut rx, &mut tx, CMUX_MAX_FRAME_SIZE),
                ppp_fut,
                self.ingress.read_from(&mut self.control_rx),
                self.cellular_runner.run(),
            )
            .await;

            self.powered = false;
        }
    }
}

pub struct ReadWriteAdapter<R, W>(pub R, pub W);

impl<R, W> embedded_io_async::ErrorType for ReadWriteAdapter<R, W> {
    type Error = ErrorKind;
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
