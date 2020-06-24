extern crate alloc;

use serialport;
use std::io;
use std::thread;

use ublox_cellular::gprs::APNInfo;
use ublox_cellular::prelude::*;
use ublox_cellular::{error::Error as GSMError, Config, GsmClient};

use atat::{self, AtatClient, ClientBuilder, ComQueue, Queues, ResQueue, UrcQueue};
use embedded_hal::digital::v2::OutputPin;
use linux_embedded_hal::Pin;
use mqttrust::{
    MqttEvent, MqttOptions, Notification, PublishRequest, QoS, Request, SubscribeRequest,
    SubscribeTopic,
};

use heapless::{consts, spsc::Queue, ArrayLength, String, Vec};

use common::{serial::Serial, timer::SysTimer};
use std::time::Duration;

fn attach_gprs<C, RST, DTR>(gsm: &GsmClient<C, RST, DTR>) -> Result<(), GSMError>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    gsm.init(true)?;
    gsm.begin().unwrap();
    gsm.attach_gprs().unwrap();
    Ok(())
}

static mut Q: Queue<Request<std::vec::Vec<u8>>, consts::U10> = Queue(heapless::i::Queue::new());

static mut URC_READY: bool = false;

struct NvicUrcMatcher {}

impl NvicUrcMatcher {
    pub fn new() -> Self {
        NvicUrcMatcher {}
    }
}

impl<BufLen: ArrayLength<u8>> atat::UrcMatcher<BufLen> for NvicUrcMatcher {
    fn process(&mut self, buf: &mut heapless::Vec<u8, BufLen>) -> atat::UrcMatcherResult<BufLen> {
        if let Some(line) = atat::get_line(buf, &[b'\r'], b'\r', b'\n', false, false) {
            unsafe { URC_READY = true };
            atat::UrcMatcherResult::Complete(line)
        } else {
            atat::UrcMatcherResult::Incomplete
        }
    }
}

type AtatRxBufLen = consts::U2048;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Serial port settings
    let settings = serialport::SerialPortSettings {
        baud_rate: 230_400,
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

    static mut RES_QUEUE: ResQueue<AtatRxBufLen, consts::U5> = Queue(heapless::i::Queue::u8());
    static mut URC_QUEUE: UrcQueue<AtatRxBufLen, consts::U10> = Queue(heapless::i::Queue::u8());
    static mut COM_QUEUE: ComQueue<consts::U3> = Queue(heapless::i::Queue::u8());

    let queues = Queues {
        res_queue: unsafe { RES_QUEUE.split() },
        urc_queue: unsafe { URC_QUEUE.split() },
        com_queue: unsafe { COM_QUEUE.split() },
    };

    let (cell_client, mut ingress) = ClientBuilder::new(
        Serial(serial_tx),
        SysTimer::new(),
        atat::Config::new(atat::Mode::Timeout),
    )
    .with_custom_urc_matcher(NvicUrcMatcher::new())
    .build(queues);

    let gsm = GsmClient::<_, Pin, Pin>::new(cell_client, Config::new(APNInfo::new("em")));

    let (mut p, c) = unsafe { Q.split() };

    // Connect to broker.hivemq.com:1883
    let mut mqtt_eventloop = MqttEvent::new(
        c,
        SysTimer::new(),
        MqttOptions::new("test_mini_1", "broker.hivemq.com".into(), 1883),
    );

    log::info!("{:?}", mqtt_eventloop.options.broker());


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
        nb::block!(mqtt_eventloop.connect(&gsm)).expect("Failed to connect to MQTT");

        // Publish @ http://www.hivemq.com/demos/websocket-client/
        p.enqueue(
            SubscribeRequest {
                topics: Vec::from_slice(&[
                    SubscribeTopic {
                        topic_path: String::from("mqttrust/tester/subscriber"),
                        qos: QoS::AtLeastOnce,
                    },
                    SubscribeTopic {
                        topic_path: String::from("mqttrust/tester/subscriber2"),
                        qos: QoS::AtLeastOnce,
                    },
                ])
                .unwrap(),
            }
            .into(),
        )
        .expect("Failed to subscribe!");

        thread::Builder::new()
            .name("eventloop".to_string())
            .spawn(move || {
                let mut cnt = 0;
                loop {
                    // Subscribe @ http://www.hivemq.com/demos/websocket-client/
                    p.enqueue(
                        PublishRequest::new(
                            String::from("fbmini/input/test_mini_1"),
                            format!("{{\"key\": \"Hello World from Factbird Mini - {}!\"}}", cnt)
                                .as_bytes()
                                .to_owned(),
                        )
                        .into(),
                    )
                    .expect("Failed to publish!");
                    cnt += 1;
                    thread::sleep(Duration::from_millis(5000));
                }
            })
            .unwrap();

        loop {
            if unsafe { URC_READY } {
                gsm.spin().unwrap();
            }
            match nb::block!(mqtt_eventloop.yield_event(&gsm)) {
                Ok(Notification::Publish(_publish)) => {
                    log::debug!(
                        "[{}, {:?}]: {:?}",
                        _publish.topic_name,
                        _publish.qospid,
                        String::from_utf8(_publish.payload).unwrap()
                    );
                }
                _ => {
                    // log::debug!("{:?}", n);
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}
