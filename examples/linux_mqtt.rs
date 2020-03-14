extern crate alloc;
extern crate atat;
extern crate env_logger;
extern crate nb;

mod common;

use serialport;
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
    gsm.begin("")?;
    gsm.attach_gprs(APNInfo::new("em"))?;
    Ok(())
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    // Serial port settings
    let settings = serialport::SerialPortSettings {
        baud_rate: 115_200,
        data_bits: serialport::DataBits::Eight,
        parity: serialport::Parity::None,
        stop_bits: serialport::StopBits::One,
        flow_control: serialport::FlowControl::None,
        timeout: Duration::from_millis(2),
    };

    // Open serial port
    let port = serialport::open_with_settings("/dev/ttyUSB0", &settings)
        .expect("Could not open serial port");
    let port2 = port.try_clone().expect("Failed to clone serial port");

    let (cell_client, mut parser) = atat::new::<_, _, SysTimer>(
        (Serial(port), Serial(port2)),
        SysTimer::new(),
        atat::Config::new(atat::Mode::Timeout),
    );

    let gsm = GSMClient::<_, Pin, Pin>::new(cell_client, GSMConfig::new());

    // Use a thread loop in lack of actual interrupt for uart rx
    let serial_irq = thread::Builder::new()
        .name("serial_irq".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(1));
            parser.handle_irq();
        })
        .unwrap();

    // let urc_handler = thread::Builder::new()
    //     .name("urc_handler".to_string())
    //     .spawn(move || loop {
    //         thread::sleep(Duration::from_millis(10));
    //         gsm.handle_urc();
    //     })
    //     .unwrap();

    if attach_gprs(&gsm).is_ok() {
        let socket = {
            let soc = gsm.open(Mode::Timeout(1000)).unwrap();
            // Upgrade socket to TLS socket
            gsm.upgrade_socket(&soc).unwrap();

            // Connect to MQTT Broker
            gsm.connect(
                soc,
                SocketAddrV4::new(Ipv4Addr::new(195, 34, 89, 241), 443).into(),
            )
            .unwrap()
        };

        let mut mqtt = MQTTClient::new(&gsm, socket, SysTimer::new(), SysTimer::new(), 512, 512);

        mqtt.connect(Connect {
            protocol: Protocol::MQTT311,
            keep_alive: 60,
            client_id: String::from("Sample_client_id"),
            clean_session: true,
            last_will: None,
            username: None,
            password: None,
        })
        .expect("Failed to connect to MQTT");

        log::info!("MQTT Connected!\r");

        loop {
            mqtt.publish(
                QoS::AtLeastOnce,
                format!("fbmini/input/{}-{}", "UUID", 0),
                "{\"key\": \"Some json payload\"}".as_bytes().to_owned(),
            )
            .expect("Failed to publish MQTT msg");
            thread::sleep(Duration::from_millis(5000));
        }
    }

    // wait for all the threads to join back (Will never happen in this example)
    // urc_handler.join().unwrap();
    serial_irq.join().unwrap();
}
