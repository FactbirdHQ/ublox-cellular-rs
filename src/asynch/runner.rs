use crate::asynch::network::NetDevice;

use crate::command::ipc::SetMultiplexing;
use crate::command::psn::DeactivatePDPContext;
use crate::command::psn::EnterPPP;

use crate::command::AT;
use crate::{command::Urc, config::CellularConfig};

use super::state;
use super::urc_handler::UrcHandler;
use super::Resources;
use crate::asynch::state::OperationState;

use atat::asynch::AtatClient;
use atat::asynch::SimpleClient;
use atat::AtatIngress as _;
use atat::UrcChannel;

use embassy_futures::select::Either;
use embassy_futures::select::Either3;
use embassy_time::with_timeout;
use embassy_time::Instant;
use embassy_time::{Duration, Timer};

use embedded_io_async::BufRead;
use embedded_io_async::Read;
use embedded_io_async::Write;

pub(crate) const URC_SUBSCRIBERS: usize = 2;

pub const CMUX_MAX_FRAME_SIZE: usize = 128;
pub const CMUX_CHANNEL_SIZE: usize = CMUX_MAX_FRAME_SIZE * 4;

#[cfg(any(feature = "internal-network-stack", feature = "ppp"))]
pub const CMUX_CHANNELS: usize = 2;

#[cfg(not(any(feature = "internal-network-stack", feature = "ppp")))]
pub const CMUX_CHANNELS: usize = 1;

/// Background runner for the Ublox Module.
///
/// You must call `.run()` in a background task for the Ublox Module to operate.
pub struct Runner<'a, R, W, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> {
    iface: (R, W),

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
    pub cmd_buf: &'a mut [u8],
    pub res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,

    #[cfg(feature = "cmux")]
    pub mux_runner: embassy_at_cmux::Runner<'a, CMUX_CHANNELS, CMUX_CHANNEL_SIZE>,

    #[cfg(feature = "cmux")]
    network_channel: embassy_at_cmux::Channel<'a, CMUX_CHANNEL_SIZE>,

    #[cfg(feature = "cmux")]
    data_channel: embassy_at_cmux::Channel<'a, CMUX_CHANNEL_SIZE>,

    #[cfg(feature = "ppp")]
    pub ppp_runner: Option<embassy_net_ppp::Runner<'a>>,
}

impl<'a, R, W, C, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Runner<'a, R, W, C, INGRESS_BUF_SIZE, URC_CAPACITY>
where
    R: BufRead + Read,
    W: Write,
    C: CellularConfig<'a> + 'a,
{
    pub fn new<const CMD_BUF_SIZE: usize>(
        iface: (R, W),
        resources: &'a mut Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
        config: C,
    ) -> Self {
        let ch_runner = state::Runner::new(&mut resources.ch);

        let ingress = atat::Ingress::new(
            atat::AtDigester::<Urc>::new(),
            &mut resources.ingress_buf,
            &resources.res_slot,
            &resources.urc_channel,
        );

        #[cfg(feature = "cmux")]
        let (mux_runner, channels) = resources.mux.start();
        #[cfg(feature = "cmux")]
        let mut channel_iter = channels.into_iter();

        Self {
            iface,

            ch: ch_runner,
            config,
            urc_channel: &resources.urc_channel,

            ingress,
            cmd_buf: &mut resources.cmd_buf,
            res_slot: &resources.res_slot,

            #[cfg(feature = "cmux")]
            mux_runner,

            #[cfg(feature = "cmux")]
            network_channel: channel_iter.next().unwrap(),

            #[cfg(feature = "cmux")]
            data_channel: channel_iter.next().unwrap(),

            #[cfg(feature = "ppp")]
            ppp_runner: None,
        }
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
        state::Device {
            shared: &self.ch.shared,
            desired_state_pub_sub: &self.ch.desired_state_pub_sub,
            urc_subscription: self.urc_channel.subscribe().unwrap(),
        }
    }

    pub async fn run<D: embassy_net::driver::Driver>(mut self, stack: &embassy_net::Stack<D>) -> ! {
        #[cfg(feature = "ppp")]
        let mut ppp_runner = self.ppp_runner.take().unwrap();

        #[cfg(feature = "cmux")]
        let (mut at_rx, mut at_tx, _) = self.network_channel.split();

        let at_config = atat::Config::default();
        loop {
            // Run the cellular device from full power down to the
            // `DataEstablished` state, handling power on, module configuration,
            // network registration & operator selection and PDP context
            // activation along the way.
            //
            // This is all done directly on the serial line, before setting up
            // virtual channels through multiplexing.
            {
                let at_client = atat::asynch::Client::new(
                    &mut self.iface.1,
                    self.res_slot,
                    self.cmd_buf,
                    at_config,
                );
                let mut cell_device = NetDevice::new(&self.ch, &mut self.config, at_client);
                let mut urc_handler = UrcHandler::new(&self.ch, self.urc_channel);

                // Clean up and start from completely powered off state. Ignore URCs in the process.
                self.ingress.clear();
                if cell_device
                    .run_to_state(OperationState::PowerDown)
                    .await
                    .is_err()
                {
                    continue;
                }

                match embassy_futures::select::select3(
                    self.ingress.read_from(&mut self.iface.0),
                    urc_handler.run(),
                    cell_device.run_to_state(OperationState::DataEstablished),
                )
                .await
                {
                    Either3::First(_) | Either3::Second(_) => {
                        // These two both have return type never (`-> !`)
                        unreachable!()
                    }
                    Either3::Third(Err(_)) => {
                        // Reboot the cellular module and try again!
                        continue;
                    }
                    Either3::Third(Ok(_)) => {
                        // All good! We are now in `DataEstablished` and ready
                        // to start communication services!
                    }
                }
            }

            #[cfg(feature = "ppp")]
            let ppp_fut = async {
                #[cfg(not(feature = "cmux"))]
                let mut iface = super::ReadWriteAdapter(&mut self.iface.0, &mut self.iface.1);

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
                            at_config,
                        );

                        let _ = at_client.send(&DeactivatePDPContext).await;

                        // hangup just in case a call was already in progress.
                        // Ignore errors because this fails if it wasn't.
                        // let _ = at_client
                        //     .send(&heapless::String::<16>::try_from("ATX0\r\n").unwrap())
                        //     .await;

                        // Send AT command to enter PPP mode
                        let res = at_client.send(&EnterPPP { cid: C::CONTEXT_ID }).await;

                        if let Err(e) = res {
                            warn!("ppp dial failed {:?}", e);
                            continue;
                        }

                        Timer::after(Duration::from_millis(100)).await;
                    }

                    // Check for CTS low (bit 2)
                    // #[cfg(feature = "cmux")]
                    // self.data_channel.set_hangup_detection(0x04, 0x00);

                    info!("RUNNING PPP");
                    let res = ppp_runner
                        .run(&mut self.data_channel, C::PPP_CONFIG, |ipv4| {
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

                    #[cfg(feature = "cmux")]
                    {
                        self.data_channel.clear_hangup_detection();

                        // escape back to data mode.
                        self.data_channel
                            .set_lines(embassy_at_cmux::Control::from_bits(0x44 << 1), None);
                        Timer::after(Duration::from_millis(100)).await;
                        self.data_channel
                            .set_lines(embassy_at_cmux::Control::from_bits(0x46 << 1), None);
                    }
                }
            };

            #[cfg(feature = "cmux")]
            let mux_fut = async {
                // Must be large enough to hold 'AT+CMUX=0,0,5,512,10,3,40,10,2\r\n'
                let mut buf = [0u8; 32];
                let mut interface = super::ReadWriteAdapter(&mut self.iface.0, &mut self.iface.1);
                {
                    let mut at_client = SimpleClient::new(
                        &mut interface,
                        atat::AtDigester::<Urc>::new(),
                        &mut buf,
                        at_config,
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
                Timer::after(Duration::from_millis(200)).await;

                // Drain the UART of any leftover AT stuff before setting up multiplexer
                let _ = embassy_time::with_timeout(Duration::from_millis(100), async {
                    loop {
                        let _ = interface.read(&mut buf).await;
                    }
                })
                .await;

                self.mux_runner
                    .run(&mut self.iface.0, &mut self.iface.1, CMUX_MAX_FRAME_SIZE)
                    .await
            };

            #[cfg(not(all(feature = "ppp", not(feature = "cmux"))))]
            let network_fut = async {
                #[cfg(not(feature = "cmux"))]
                let (mut at_rx, mut at_tx) = self.iface;

                let at_client =
                    atat::asynch::Client::new(&mut at_tx, self.res_slot, self.cmd_buf, at_config);
                let mut cell_device = NetDevice::new(&self.ch, &mut self.config, at_client);

                let mut urc_handler = UrcHandler::new(&self.ch, self.urc_channel);

                // TODO: Should we set ATE0 and CMEE=1 here, again?

                embassy_futures::join::join3(
                    self.ingress.read_from(&mut at_rx),
                    cell_device.run(),
                    urc_handler.run(),
                )
                .await;
            };

            #[cfg(all(feature = "ppp", not(feature = "cmux")))]
            ppp_fut.await;

            #[cfg(all(feature = "ppp", feature = "cmux"))]
            match embassy_futures::select::select3(mux_fut, ppp_fut, network_fut).await {
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

            #[cfg(all(feature = "cmux", not(feature = "ppp")))]
            match embassy_futures::select::select(mux_fut, network_fut).await {
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
