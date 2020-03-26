extern crate alloc;
extern crate atat;
extern crate env_logger;
extern crate nb;
mod common;

use serialport;
use std::io;
use std::sync::Arc;
use std::thread;

use ublox_cellular::gprs::APNInfo;
use ublox_cellular::prelude::*;
use ublox_cellular::soc::{Ipv4Addr, Mode, SocketAddrV4};
use ublox_cellular::{error::Error as GSMError, GSMClient, GSMConfig};

use atat::AtatClient;
use embedded_hal::digital::v2::OutputPin;
use linux_embedded_hal::Pin;
use mqttrust::{Connect, MQTTClient, Protocol, QoS};

use common::{serial::Serial, timer::SysTimer};
use std::time::Duration;

fn attach_gprs<C, RST, DTR>(gsm: &GSMClient<C, RST, DTR>) -> Result<(), GSMError>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    gsm.init(true)?;

    gsm.import_root_ca(0, "Verisign", include_str!("./secrets/aws/Verisign.pem"))
        .expect("Failed to import root CA");

    gsm.import_certificate(
        0,
        "cf0c600_cert",
        include_str!("./secrets/aws/certificate.pem.crt"),
    )
    .expect("Failed to import certificate");

    gsm.import_private_key(
        0,
        "cf0c600_key",
        include_str!("./secrets/aws/private.pem.key"),
        None,
    )
    .expect("Failed to import private key");

    gsm.begin("").unwrap();
    gsm.attach_gprs(APNInfo::new("em")).unwrap();
    Ok(())
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Serial port settings
    let settings = serialport::SerialPortSettings {
        baud_rate: 115_200,
        data_bits: serialport::DataBits::Eight,
        parity: serialport::Parity::None,
        stop_bits: serialport::StopBits::One,
        flow_control: serialport::FlowControl::None,
        timeout: Duration::from_millis(5000),
    };

    // Open serial port
    let serial_tx = serialport::open_with_settings("/dev/ttyUSB0", &settings)
        .expect("Could not open serial port");
    let mut serial_rx = serial_tx.try_clone().expect("Failed to clone serial port");

    let (cell_client, mut ingress) = atat::new::<_, SysTimer, atat::NoopUrcMatcher>(
        Serial(serial_tx),
        SysTimer::new(),
        atat::Config::new(atat::Mode::Timeout),
        None,
    );

    let gsm = Arc::new(GSMClient::<_, Pin, Pin>::new(cell_client, GSMConfig::new()));
    // Launch reading thread
    thread::Builder::new()
        .name("serial_read".to_string())
        .spawn(move || loop {
            let mut buffer = [0; 32];
            match serial_rx.read(&mut buffer[..]) {
                Ok(0) => {}
                Ok(bytes_read) => {
                    ingress.write(&buffer[0..bytes_read]);
                    ingress.digest();
                    ingress.digest();
                    // gsm.spin();
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock
                    | io::ErrorKind::TimedOut
                    | io::ErrorKind::Interrupted => {
                        // Ignore
                    }
                    _ => {
                        log::error!("Serial reading thread error while reading: {}", e);
                    }
                },
            }
        })
        .unwrap();

    if attach_gprs(&gsm).is_ok() {
        let socket = {
            let soc = gsm.open(Mode::Timeout(1000)).unwrap();

            let ip = gsm
                .dns_lookup("a69ih9fwq4cti.iot.eu-west-1.amazonaws.com")
                .unwrap();
            // Connect to MQTT Broker: a69ih9fwq4cti-ats.iot.eu-west-1.amazonaws.com
            gsm.connect(
                soc,
                // a69ih9fwq4cti.iot.eu-west-1.amazonaws.com :
                SocketAddrV4::new(ip, 8883).into(),
                // a69ih9fwq4cti-ats.iot.eu-west-1.amazonaws.com :
                // SocketAddrV4::new(Ipv4Addr::new(34, 250, 137, 90), 8883).into(),
                // test.mosquitto.org :
                // SocketAddrV4::new(Ipv4Addr::new(5, 196, 95, 208), 8884).into(),
            )
            .unwrap()
        };

        let mut mqtt = MQTTClient::new(SysTimer::new(), SysTimer::new(), 512, 512);

        nb::block!(mqtt.connect(
            &*gsm,
            socket,
            Connect {
                protocol: Protocol::MQTT311,
                keep_alive: 60,
                client_id: String::from("MINI"),
                clean_session: true,
                last_will: None,
                username: None,
                password: None,
            }
        ))
        .expect("Failed to connect to MQTT");

        log::info!("MQTT Connected!");

        let mut cnt = 0;
        loop {
            nb::block!(mqtt.publish(
                &*gsm,
                QoS::AtLeastOnce,
                String::from("fbmini"),
                format!("{{\"key\": \"Hello World from Factbird Mini - {}!\"}}", cnt)
                    .as_bytes()
                    .to_owned(),
            ))
            .expect("Failed to publish MQTT msg");
            cnt += 1;
            thread::sleep(Duration::from_millis(5000));
        }
    }
}
