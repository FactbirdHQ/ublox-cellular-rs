#![cfg(feature = "ppp")]

use atat::asynch::AtatClient as _;
use atat::asynch::SimpleClient;
use atat::AtatIngress as _;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::TcpClient;
use embassy_net::tcp::client::TcpClientState;
use embassy_net::tcp::TcpSocket;
use embassy_net::Stack;
use embassy_net::StackResources;

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Duration;
use embassy_time::Instant;
use embassy_time::Timer;
use embedded_mqtt::transport::embedded_tls::TlsNalTransport;
use embedded_mqtt::transport::embedded_tls::TlsState;
use embedded_mqtt::DomainBroker;
use embedded_tls::Aes128GcmSha256;
use embedded_tls::Certificate;
use embedded_tls::TlsConfig;
use embedded_tls::TlsConnection;
use embedded_tls::TlsContext;
use embedded_tls::UnsecureProvider;
use log::*;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use reqwless::headers::ContentType;
use reqwless::request::Request;
use reqwless::request::RequestBuilder as _;
use reqwless::response::Response;
use static_cell::StaticCell;
use tokio_serial::SerialPort;
use tokio_serial::SerialPortBuilderExt;
use ublox_cellular::asynch::state::OperationState;
use ublox_cellular::asynch::Resources;

use ublox_cellular::command::control::SetDataRate;
use ublox_cellular::command::control::SetFlowControl;
use ublox_cellular::command::general::GetModelId;
use ublox_cellular::command::ipc::SetMultiplexing;
use ublox_cellular::command::psn::DeactivatePDPContext;
use ublox_cellular::command::psn::EnterPPP;
use ublox_cellular::command::Urc;
use ublox_cellular::command::AT;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("HELLO");

    let mut ppp_iface = tokio_serial::new(TTY, 115200).open_native_async()?;
    ppp_iface
        .set_flow_control(tokio_serial::FlowControl::Hardware)
        .unwrap();

    static RESOURCES: StaticCell<Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>> =
        StaticCell::new();

    let (net_device, mut cell_control, mut runner) =
        ublox_cellular::asynch::new_ppp(RESOURCES.init(Resources::new()), MyCelullarConfig);

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

    let http_fut = async {
        stack.wait_config_up().await;

        info!("We have network!");

        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        let hostname = "ecdsa-test.germancoding.com";

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

        let mut read_record_buffer = [0; 16384];
        let mut write_record_buffer = [0; 16384];
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

        let mut buf = vec![0; 16384];
        let len = response
            .body()
            .reader()
            .read_to_end(&mut buf)
            .await
            .unwrap();
        info!("{:?}", core::str::from_utf8(&buf[..len]));
    };

    let (rx, tx) = tokio::io::split(ppp_iface);
    let rx = embedded_io_adapters::tokio_1::FromTokio::new(tokio::io::BufReader::new(rx));
    let tx = embedded_io_adapters::tokio_1::FromTokio::new(tx);
    embassy_futures::join::join3(
        stack.run(),
        runner.run(rx, tx, |ipv4| {
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
        http_fut,
    )
    .await;

    unreachable!();
}
