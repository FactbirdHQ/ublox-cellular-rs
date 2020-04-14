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
use ublox_cellular::{error::Error as GSMError, GSMClient, GSMConfig};

use atat::AtatClient;
use embedded_hal::digital::v2::OutputPin;
use linux_embedded_hal::Pin;
use mqttrust::{MqttEvent, MqttOptions, Notification, PublishRequest, Request};

use heapless::{consts, spsc::Queue};

use common::{serial::Serial, timer::SysTimer};
use std::time::Duration;

fn attach_gprs<C, RST, DTR>(gsm: &GSMClient<C, RST, DTR>) -> Result<(), GSMError>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    gsm.init(true)?;
    gsm.begin("").unwrap();
    gsm.attach_gprs(APNInfo::new("em")).unwrap();
    Ok(())
}

static mut Q: Queue<Request, consts::U10> = Queue(heapless::i::Queue::new());

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

    let (mut p, c) = unsafe { Q.split() };

    let gsm = GSMClient::<_, Pin, Pin>::new(cell_client, GSMConfig::new());

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
                        #[cfg(features = "logging")]
                        log::error!("Serial reading thread error while reading: {}", e);
                    }
                },
            }
        })
        .unwrap();

    if attach_gprs(&gsm).is_ok() {
        let ip = gsm.dns_lookup("test.mosquitto.org").unwrap();

        let mut mqtt_eventloop = MqttEvent::new(
            c,
            SysTimer::new(),
            SysTimer::new(),
            MqttOptions::new("mqtt_test_client_id", ip, 1883),
        );

        nb::block!(mqtt_eventloop.connect(&gsm)).expect("Failed to connect to MQTT");

        thread::Builder::new()
            .name("eventloop".to_string())
            .spawn(move || loop {
                match nb::block!(mqtt_eventloop.yield_event(&gsm)) {
                    Ok(Notification::Publish(publish)) => {
                        #[cfg(features = "logging")]
                        log::debug!(
                            "[{}, {:?}]: {:?}",
                            publish.topic_name,
                            publish.qospid,
                            String::from_utf8(publish.payload).unwrap()
                        );
                    }
                    _ => {
                        // #[cfg(features = "logging")]
                        // log::debug!("{:?}", n);
                    }
                }
            })
            .unwrap();

        #[cfg(features = "logging")]
        log::info!("MQTT Connected!");

        let mut cnt = 0;
        loop {
            p.enqueue(
                PublishRequest::new(
                    "ublox_mqtt/tester/whatup".to_owned(),
                    format!("{{\"key\": \"Hello World from UBlox - {}!\"}}", cnt)
                        .as_bytes()
                        .to_owned(),
                )
                .into(),
            )
            .expect("Failed to publish!");
            cnt += 1;
            thread::sleep(Duration::from_millis(5000));
        }
    }
}
