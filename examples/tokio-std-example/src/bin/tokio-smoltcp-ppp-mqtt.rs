#![cfg(feature = "ppp")]

use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::TcpClient;
use embassy_net::tcp::client::TcpClientState;
use embassy_net::Stack;
use embassy_net::StackResources;

use embassy_net_ppp::Device;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Duration;
use embedded_mqtt::transport::embedded_tls::TlsNalTransport;
use embedded_mqtt::transport::embedded_tls::TlsState;
use embedded_mqtt::DomainBroker;
use embedded_tls::Aes128GcmSha256;
use embedded_tls::Certificate;
use embedded_tls::TlsConfig;
use embedded_tls::UnsecureProvider;
use log::*;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use static_cell::StaticCell;
use tokio_serial::SerialPort;
use tokio_serial::SerialPortBuilderExt;
use ublox_cellular::asynch::Resources;

use ublox_cellular::config::NoPin;
use ublox_cellular::config::{Apn, CellularConfig};

const CMD_BUF_SIZE: usize = 128;
const INGRESS_BUF_SIZE: usize = 512;
const URC_CAPACITY: usize = 2;

struct MyCelullarConfig;

impl<'a> CellularConfig<'a> for MyCelullarConfig {
    type ResetPin = NoPin;
    type PowerPin = NoPin;
    type VintPin = NoPin;

    const FLOW_CONTROL: bool = true;

    const APN: Apn<'a> = Apn::Given {
        name: "em",
        username: None,
        password: None,
    };

    const PPP_CONFIG: embassy_net_ppp::Config<'a> = embassy_net_ppp::Config {
        username: b"",
        password: b"",
    };
}

const TTY: &str = "/dev/ttyUSB0";
const HOSTNAME: &str = "a2twqv2u8qs5xt-ats.iot.eu-west-1.amazonaws.com";
const MQTT_MAX_SUBSCRIBERS: usize = 2;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("HELLO");

    let mut ppp_iface = tokio_serial::new(TTY, 115200).open_native_async()?;
    ppp_iface
        .set_flow_control(tokio_serial::FlowControl::Hardware)
        .unwrap();

    let (rx, tx) = tokio::io::split(ppp_iface);
    let rx = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::BufReader::new(rx));
    let tx = embedded_io_adapters::tokio_1::FromTokio::new(tx);

    static RESOURCES: StaticCell<Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>> =
        StaticCell::new();
    let mut runner = ublox_cellular::asynch::Runner::new(
        (rx, tx),
        RESOURCES.init(Resources::new()),
        MyCelullarConfig,
    );

    static PPP_STATE: StaticCell<embassy_net_ppp::State<2, 2>> = StaticCell::new();
    let net_device = runner.ppp_stack(PPP_STATE.init(embassy_net_ppp::State::new()));

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    static STACK: StaticCell<Stack<embassy_net_ppp::Device<'static>>> = StaticCell::new();
    static STACK_RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();

    let stack = &*STACK.init(Stack::new(
        net_device,
        embassy_net::Config::default(),
        STACK_RESOURCES.init(StackResources::new()),
        seed,
    ));

    let mqtt_fut = async {
        stack.wait_config_up().await;

        info!("We have network!");

        static DNS: StaticCell<DnsSocket<Device>> = StaticCell::new();
        let broker = DomainBroker::<_, 64>::new(HOSTNAME, DNS.init(DnsSocket::new(stack))).unwrap();

        static MQTT_STATE: StaticCell<
            embedded_mqtt::State<NoopRawMutex, 4096, 4096, MQTT_MAX_SUBSCRIBERS>,
        > = StaticCell::new();

        let (mut mqtt_stack, mqtt_client) = embedded_mqtt::new(
            MQTT_STATE.init(embedded_mqtt::State::new()),
            embedded_mqtt::Config::new("csr_test", broker)
                .keepalive_interval(Duration::from_secs(50)),
        );

        let mqtt_tcp_state = TcpClientState::<1, 4096, 4096>::new();
        let mqtt_tcp_client = TcpClient::new(stack, &mqtt_tcp_state);

        let provider = UnsecureProvider::new::<Aes128GcmSha256>(ChaCha8Rng::seed_from_u64(seed));

        let tls_config = TlsConfig::new()
            .with_server_name(HOSTNAME)
            // .with_ca(Certificate::X509(include_bytes!(
            //     "/home/mathias/Downloads/AmazonRootCA3.cer"
            // )))
            .with_cert(Certificate::X509(include_bytes!(
                "/home/mathias/Downloads/embedded-tls-test-certs/cert.der"
            )))
            .with_priv_key(include_bytes!(
                "/home/mathias/Downloads/embedded-tls-test-certs/private.der"
            ));

        let tls_state = TlsState::<16640, 16640>::new();
        let mut transport =
            TlsNalTransport::new(&mqtt_tcp_client, &tls_state, &tls_config, provider);

        mqtt_stack.run(&mut transport).await;
    };

    embassy_futures::join::join3(
        stack.run(),
        runner.run(|ipv4| {
            let Some(addr) = ipv4.address else {
                warn!("PPP did not provide an IP address.");
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
        }),
        mqtt_fut,
    )
    .await;

    unreachable!();
}
