#![no_std]
#![no_main]
#![allow(stable_features)]
#![feature(type_alias_impl_trait)]

use atat::asynch::AtatClient;
use atat::asynch::Client;

use atat::asynch::SimpleClient;
use atat::ResponseSlot;
use atat::UrcChannel;

use cortex_m_rt::entry;
use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_futures::join::join;
use embassy_net::tcp::TcpSocket;
use embassy_net::ConfigV4;
use embassy_net::Ipv4Address;
use embassy_net::Ipv4Cidr;
use embassy_net::Stack;
use embassy_net::StackResources;
use embassy_rp::gpio;
use embassy_rp::gpio::Input;

use embassy_rp::gpio::OutputOpenDrain;
use embassy_rp::uart::BufferedUart;
use embassy_rp::{bind_interrupts, peripherals::UART0, uart::BufferedInterruptHandler};
use embassy_time::Instant;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use ublox_cellular::asynch::state::OperationState;
use ublox_cellular::command::control::types::Circuit108Behaviour;
use ublox_cellular::command::control::types::Circuit109Behaviour;
use ublox_cellular::command::control::types::FlowControl;
use ublox_cellular::command::control::SetCircuit108Behaviour;
use ublox_cellular::command::control::SetCircuit109Behaviour;
use ublox_cellular::command::control::SetFlowControl;
use ublox_cellular::command::general::GetCCID;
use ublox_cellular::command::general::GetFirmwareVersion;
use ublox_cellular::command::general::GetModelId;
use ublox_cellular::command::gpio::types::GpioInPull;
use ublox_cellular::command::gpio::types::GpioMode;
use ublox_cellular::command::gpio::types::GpioOutValue;
use ublox_cellular::command::gpio::SetGpioConfiguration;
use ublox_cellular::command::ipc::SetMultiplexing;
use ublox_cellular::command::mobile_control::types::Functionality;
use ublox_cellular::command::mobile_control::types::ResetMode;
use ublox_cellular::command::mobile_control::types::TerminationErrorMode;
use ublox_cellular::command::mobile_control::SetModuleFunctionality;
use ublox_cellular::command::mobile_control::SetReportMobileTerminationError;
use ublox_cellular::command::psn::types::ContextId;
use ublox_cellular::command::psn::DeactivatePDPContext;
use ublox_cellular::command::psn::EnterPPP;
use ublox_cellular::command::psn::SetPDPContextDefinition;
use ublox_cellular::command::system_features::types::PowerSavingMode;
use ublox_cellular::command::system_features::SetPowerSavingControl;
use {defmt_rtt as _, panic_probe as _};

use embassy_at_cmux::Mux;

use ublox_cellular::config::{Apn, CellularConfig};

use atat::{AtDigester, AtatIngress};
use ublox_cellular::asynch::State;
use ublox_cellular::command::Urc;

bind_interrupts!(struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

const INGRESS_BUF_SIZE: usize = 512;
const URC_CAPACITY: usize = 2;
const URC_SUBSCRIBERS: usize = 2;

struct Networking {
    ppp: PPP,
    mux: GsmMux,
    cellular_runner: ublox_cellular::asynch::runner::Runner<
        'static,
        Client<'static, embassy_at_cmux::ChannelTx<'static, 256>, INGRESS_BUF_SIZE>,
        MyCelullarConfig,
        2,
    >,
    btn: embassy_rp::gpio::Input<'static>,
}

impl Networking {
    pub async fn run(mut self) -> ! {
        let fut = async {
            loop {
                // Reboot modem and start again
                Timer::after(Duration::from_secs(5)).await;
                embassy_futures::select::select3(
                    self.cellular_runner.run(),
                    self.ppp.run(),
                    self.btn.wait_for_any_edge(),
                )
                .await;

                let _ = self.cellular_runner.reset().await;
            }
        };
        join(self.mux.run(), fut).await;
        core::unreachable!();
    }
}

struct PPP {
    stack: &'static Stack<embassy_net_ppp::Device<'static>>,
    ppp_runner: embassy_net_ppp::Runner<'static>,
    ppp_channel: embassy_at_cmux::Channel<'static, 256>,
}

impl PPP {
    async fn configure<A: AtatClient>(at_client: &mut A) -> Result<(), atat::Error> {
        at_client
            .send(&SetModuleFunctionality {
                fun: Functionality::Minimum,
                rst: Some(ResetMode::DontReset),
            })
            .await?;

        at_client
            .send(&SetPDPContextDefinition {
                cid: ContextId(1),
                pdp_type: "IP",
                apn: "em",
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

    async fn run(&mut self) {
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
                        return;
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

            if let Err(e) = Self::configure(&mut at_client).await {
                warn!("modem: configure failed {:?}", e);
                continue;
            }

            Timer::after(Duration::from_secs(2)).await;

            // hangup just in case a call was already in progress.
            // Ignore errors because this fails if it wasn't.
            let _ = at_client.send(&DeactivatePDPContext).await;

            // Send AT command to enter PPP mode
            let res = at_client.send(&EnterPPP { cid: ContextId(1) }).await;

            if let Err(e) = res {
                warn!("ppp dial failed {:?}", e);
                continue;
            }

            drop(at_client);

            // Check for CTS low (bit 2)
            self.ppp_channel.set_hangup_detection(0x04, 0x00);

            info!("RUNNING PPP");
            let config = embassy_net_ppp::Config {
                username: b"",
                password: b"",
            };
            let res = self
                .ppp_runner
                .run(&mut self.ppp_channel, config, |ipv4| {
                    let Some(addr) = ipv4.address else {
                        warn!("PPP did not provide an IP address.");
                        return;
                    };
                    let mut dns_servers = heapless::Vec::new();
                    for s in ipv4.dns_servers.iter().flatten() {
                        let _ = dns_servers.push(Ipv4Address::from_bytes(&s.0));
                    }
                    let config = ConfigV4::Static(embassy_net::StaticConfigV4 {
                        address: Ipv4Cidr::new(Ipv4Address::from_bytes(&addr.0), 0),
                        gateway: None,
                        dns_servers,
                    });
                    self.stack.set_config_v4(config);
                })
                .await;

            info!("ppp failed: {:?}", res);

            self.ppp_channel.clear_hangup_detection();

            // escape back to data mode.
            self.ppp_channel.set_lines(0x44);
            Timer::after(Duration::from_millis(100)).await;
            self.ppp_channel.set_lines(0x46);
        }
    }
}

struct GsmMux {
    mux_runner: embassy_at_cmux::Runner<'static, 2, 256>,
    iface: BufferedUart<'static, UART0>,
}

impl GsmMux {
    async fn init(&mut self) -> Result<(), atat::Error> {
        let mut buf = [0u8; 64];
        let mut at_client = SimpleClient::new(
            &mut self.iface,
            atat::AtDigester::<Urc>::new(),
            &mut buf,
            atat::Config::default(),
        );

        at_client
            .send(&SetReportMobileTerminationError {
                n: TerminationErrorMode::Enabled,
            })
            .await?;

        // Select SIM
        at_client
            .send(&SetGpioConfiguration {
                gpio_id: 25,
                gpio_mode: GpioMode::Output(GpioOutValue::High),
            })
            .await?;

        at_client
            .send(&SetGpioConfiguration {
                gpio_id: 42,
                gpio_mode: GpioMode::Input(GpioInPull::NoPull),
            })
            .await?;

        let _model_id = at_client.send(&GetModelId).await?;

        // at_client.send(
        //     &IdentificationInformation {
        //         n: 9
        //     },
        // ).await?;

        at_client.send(&GetFirmwareVersion).await?;

        // self.select_sim_card().await?;

        let ccid = at_client.send(&GetCCID).await?;
        info!("CCID: {}", ccid.ccid);

        // DCD circuit (109) changes in accordance with the carrier
        at_client
            .send(&SetCircuit109Behaviour {
                value: Circuit109Behaviour::ChangesWithCarrier,
            })
            .await?;

        // Ignore changes to DTR
        at_client
            .send(&SetCircuit108Behaviour {
                value: Circuit108Behaviour::Ignore,
            })
            .await?;

        // Switch off UART power saving until it is integrated into this API
        at_client
            .send(&SetPowerSavingControl {
                mode: PowerSavingMode::Disabled,
                timeout: None,
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
    pub async fn run(mut self) {
        self.init().await.unwrap();

        let (mut rx, mut tx) = self.iface.split();
        self.mux_runner.run(&mut rx, &mut tx).await
    }
}

struct MyCelullarConfig {
    reset_pin: Option<OutputOpenDrain<'static>>,
    power_pin: Option<OutputOpenDrain<'static>>,
    vint_pin: Option<Input<'static>>,
}

impl<'a> CellularConfig<'a> for MyCelullarConfig {
    type ResetPin = OutputOpenDrain<'static>;
    type PowerPin = OutputOpenDrain<'static>;
    type VintPin = Input<'static>;

    const FLOW_CONTROL: bool = true;

    const APN: Apn<'a> = Apn::Given {
        name: "em",
        username: None,
        password: None,
    };

    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        info!("reset_pin");
        self.reset_pin.as_mut()
    }
    fn power_pin(&mut self) -> Option<&mut Self::PowerPin> {
        info!("power_pin");
        self.power_pin.as_mut()
    }
    fn vint_pin(&mut self) -> Option<&mut Self::VintPin> {
        info!("vint_pin = {}", self.vint_pin.as_mut()?.is_high());
        self.vint_pin.as_mut()
    }
}

static RES_SLOT: ResponseSlot<INGRESS_BUF_SIZE> = ResponseSlot::new();
static URC_CHANNEL: UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS> = UrcChannel::new();

#[embassy_executor::task]
async fn main_task(spawner: Spawner) {
    let p = {
        let config =
            embassy_rp::config::Config::new(embassy_rp::clocks::ClockConfig::crystal(12_000_000));
        embassy_rp::init(config)
    };

    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();

    let mut cell_uart_config = embassy_rp::uart::Config::default();
    cell_uart_config.baudrate = 115200;

    let cell_uart = BufferedUart::new_with_rtscts(
        p.UART0,
        Irqs,
        p.PIN_0,
        p.PIN_1,
        p.PIN_3,
        p.PIN_2,
        TX_BUF.init([0; 16]),
        RX_BUF.init([0; 16]),
        cell_uart_config,
    );

    let cell_nrst = gpio::OutputOpenDrain::new(p.PIN_4, gpio::Level::High);
    let cell_pwr = gpio::OutputOpenDrain::new(p.PIN_5, gpio::Level::High);
    let cell_vint = gpio::Input::new(p.PIN_6, gpio::Pull::None);

    // Create new `embassy-net-ppp` device and runner pair
    static PPP_STATE: StaticCell<embassy_net_ppp::State<2, 2>> = StaticCell::new();
    let (net_device, ppp_runner) =
        embassy_net_ppp::new(PPP_STATE.init(embassy_net_ppp::State::new()));

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<embassy_net_ppp::Device<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();

    let stack = &*STACK.init(Stack::new(
        net_device,
        embassy_net::Config::default(),
        RESOURCES.init(StackResources::new()),
        seed,
    ));

    static CMUX: StaticCell<Mux<2, 256>> = StaticCell::new();

    let mux = CMUX.init(Mux::new());
    let (mux_runner, [ch1, ch2]) = mux.start();

    let (control_rx, control_tx, _) = ch2.split();
    static CMD_BUF: StaticCell<[u8; 128]> = StaticCell::new();
    let at_client = Client::new(
        control_tx,
        &RES_SLOT,
        CMD_BUF.init([0; 128]),
        atat::Config::default(),
    );

    spawner.spawn(ingress_task(control_rx)).unwrap();

    static STATE: StaticCell<
        State<Client<embassy_at_cmux::ChannelTx<'static, 256>, INGRESS_BUF_SIZE>>,
    > = StaticCell::new();
    let (mut control, runner) = ublox_cellular::asynch::new_ppp(
        STATE.init(State::new(at_client)),
        &URC_CHANNEL,
        MyCelullarConfig {
            reset_pin: Some(cell_nrst),
            power_pin: Some(cell_pwr),
            vint_pin: Some(cell_vint),
        },
    );

    control.set_desired_state(OperationState::Alive).await;

    let networking = Networking {
        mux: GsmMux {
            iface: cell_uart,
            mux_runner,
        },
        ppp: PPP {
            stack,
            ppp_runner,
            ppp_channel: ch1,
        },
        cellular_runner: runner,
        btn: gpio::Input::new(p.PIN_27, gpio::Pull::Up),
    };

    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(ppp_task(networking)).unwrap();

    Timer::after(Duration::from_secs(15)).await;

    control.set_desired_state(OperationState::Connected).await;

    stack.wait_config_up().await;

    info!("WE have network!");

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

    socket.set_timeout(Some(Duration::from_secs(10)));

    let remote_endpoint = (Ipv4Address::new(93, 184, 216, 34), 80);
    info!("connecting to {:?}...", remote_endpoint);
    let r = socket.connect(remote_endpoint).await;
    if let Err(e) = r {
        warn!("connect error: {:?}", e);
        return;
    }
    info!("TCP connected!");
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<embassy_net_ppp::Device<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn ppp_task(networking: Networking) -> ! {
    networking.run().await
}

#[embassy_executor::task]
async fn ingress_task(mut rx: embassy_at_cmux::ChannelRx<'static, 256>) -> ! {
    let mut buf = [0u8; INGRESS_BUF_SIZE];

    let mut ingress =
        atat::Ingress::new(AtDigester::<Urc>::new(), &mut buf, &RES_SLOT, &URC_CHANNEL);
    loop {
        let buf = ingress.write_buf();
        match embedded_io_async::Read::read(&mut rx, buf).await {
            Ok(received) => {
                // Ignore errors, as they mean the URC channel was full. This will be automatically redriven later
                if ingress.try_advance(received).is_err() {
                    Timer::after(Duration::from_millis(100)).await;
                    ingress.try_advance(0).ok();
                }
            }
            Err(e) => {
                defmt::error!(
                    "Got serial read error {:?}",
                    embedded_io_async::Error::kind(&e)
                );
                ingress.clear();
            }
        }
    }
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("Hello World!");

    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        unwrap!(spawner.spawn(main_task(spawner)));
    })
}
