#![cfg(feature = "ppp")]
#![no_std]
#![no_main]
#![allow(stable_features)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_net::StackResources;
use embassy_rp::gpio;
use embassy_rp::gpio::Input;

use embassy_rp::gpio::OutputOpenDrain;
use embassy_rp::uart::BufferedUart;
use embassy_rp::uart::BufferedUartRx;
use embassy_rp::uart::BufferedUartTx;
use embassy_rp::{bind_interrupts, peripherals::UART0, uart::BufferedInterruptHandler};
use embassy_time::Duration;
use embedded_tls::Aes128GcmSha256;
use embedded_tls::TlsConfig;
use embedded_tls::TlsConnection;
use embedded_tls::TlsContext;
use embedded_tls::UnsecureProvider;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use reqwless::headers::ContentType;
use reqwless::request::Request;
use reqwless::request::RequestBuilder as _;
use reqwless::response::Response;
use static_cell::StaticCell;
use ublox_cellular::asynch::Resources;
use {defmt_rtt as _, panic_probe as _};

use ublox_cellular::config::{Apn, CellularConfig};

bind_interrupts!(struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

const CMD_BUF_SIZE: usize = 128;
const INGRESS_BUF_SIZE: usize = 512;
const URC_CAPACITY: usize = 16;

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
        name: "onomondo",
        username: None,
        password: None,
    };

    const PPP_CONFIG: embassy_net_ppp::Config<'a> = embassy_net_ppp::Config {
        username: b"",
        password: b"",
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = {
        let config =
            embassy_rp::config::Config::new(embassy_rp::clocks::ClockConfig::crystal(12_000_000));
        embassy_rp::init(config)
    };

    static TX_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 256]> = StaticCell::new();

    let cell_uart = BufferedUart::new_with_rtscts(
        p.UART0,
        Irqs,
        p.PIN_0,
        p.PIN_1,
        p.PIN_3,
        p.PIN_2,
        TX_BUF.init([0; 256]),
        RX_BUF.init([0; 256]),
        embassy_rp::uart::Config::default(),
    );

    let cell_nrst = gpio::OutputOpenDrain::new(p.PIN_4, gpio::Level::High);
    let cell_pwr = gpio::OutputOpenDrain::new(p.PIN_5, gpio::Level::High);
    let cell_vint = gpio::Input::new(p.PIN_6, gpio::Pull::None);

    static RESOURCES: StaticCell<Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>> =
        StaticCell::new();
    let mut runner = ublox_cellular::asynch::Runner::new(
        cell_uart.split(),
        RESOURCES.init(Resources::new()),
        MyCelullarConfig {
            reset_pin: Some(cell_nrst),
            power_pin: Some(cell_pwr),
            vint_pin: Some(cell_vint),
        },
    );

    static PPP_STATE: StaticCell<embassy_net_ppp::State<2, 2>> = StaticCell::new();
    let net_device = runner.ppp_stack(PPP_STATE.init(embassy_net_ppp::State::new()));

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<embassy_net_ppp::Device<'static>>> = StaticCell::new();
    static STACK_RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();

    let stack = &*STACK.init(Stack::new(
        net_device,
        embassy_net::Config::default(),
        STACK_RESOURCES.init(StackResources::new()),
        seed,
    ));

    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(cell_task(runner, stack)).unwrap();

    stack.wait_config_up().await;

    // embassy_time::Timer::after(Duration::from_secs(2)).await;

    info!("We have network!");

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(20)));

   
    let hostname = "ecdsa-test.germancoding.com";
    // let hostname = "eohkv57m7xxdr4m.m.pipedream.net";
    info!("looking up {:?}...", hostname);

    let mut remote = stack
        .dns_query(hostname, smoltcp::wire::DnsQueryType::A)
        .await
        .unwrap();
    let remote_endpoint = (remote.pop().unwrap(), 443);
    info!("connecting to {:?}...", remote_endpoint);
    let r = socket.connect(remote_endpoint).await;
    if let Err(e) = r {
        warn!("connect error: {:?}", e);
        return;
    }
    info!("TCP connected!");

    let mut read_record_buffer = [0; 16640];
    let mut write_record_buffer = [0; 16640];
    let config = TlsConfig::new().with_server_name(hostname);
    let mut tls = TlsConnection::new(socket, &mut read_record_buffer, &mut write_record_buffer);

    tls.open(TlsContext::new(
        &config,
        UnsecureProvider::new::<Aes128GcmSha256>(ChaCha8Rng::seed_from_u64(seed)),
    ))
    .await
    .expect("error establishing TLS connection");

    info!("TLS Established!");

    let request = Request::get("/")
        .host(hostname)
        .content_type(ContentType::TextPlain)
        .build();
    request.write(&mut tls).await.unwrap();

    let mut rx_buf = [0; 4096];
    let response = Response::read(&mut tls, reqwless::request::Method::GET, &mut rx_buf)
        .await
        .unwrap();

    {
            info!("Got resp! {=[u8]:a}", &rx_buf[..512]);

        }

    // let mut buf = [0; 16384];
    // let len = response
    //     .body()
    //     .reader()
    //     .read_to_end(&mut buf)
    //     .await
    //     .unwrap();
    // info!("{=[u8]:a}", &buf[..len]);

    loop {
        embassy_time::Timer::after(Duration::from_secs(1)).await
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<embassy_net_ppp::Device<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn cell_task(
    runner: ublox_cellular::asynch::Runner<
        'static,
        BufferedUartRx<'static, UART0>,
        BufferedUartTx<'static, UART0>,
        MyCelullarConfig,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
    >,
    stack: &'static embassy_net::Stack<embassy_net_ppp::Device<'static>>,
) -> ! {
    runner
        .run(|ipv4| {
            let Some(addr) = ipv4.address else {
                defmt::warn!("PPP did not provide an IP address.");
                return;
            };
            let mut dns_servers = heapless::Vec::new();
            for s in ipv4.dns_servers.iter().flatten() {
                let _ = dns_servers.push(embassy_net::Ipv4Address::from_bytes(&s.0));
            }
            let config = embassy_net::ConfigV4::Static(embassy_net::StaticConfigV4 {
                address: embassy_net::Ipv4Cidr::new(
                    embassy_net::Ipv4Address::from_bytes(&addr.0),
                    0,
                ),
                gateway: None,
                dns_servers,
            });

            stack.set_config_v4(config);
        })
        .await
}
