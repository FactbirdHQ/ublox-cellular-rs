extern crate alloc;

use embedded_nal::nb;
use std::{io, thread, time::Duration};

use atat::{ClientBuilder, ComQueue, Queues, ResQueue, UrcQueue};
use mqttrust::{
    Mqtt, MqttClient, MqttEvent, MqttOptions, Notification, QoS, Request, SubscribeTopic,
};
use ublox_cellular::{sockets::SocketSet, APNInfo, Apn, Config, ContextId, GsmClient, ProfileId};

use heapless::{consts, spsc::Queue, ArrayLength};

use common::{serial::serialport, serial::Serial, timer::SysTimer};

static mut Q: Queue<Request<heapless::Vec<u8, consts::U2048>>, consts::U10, u8> =
    Queue(heapless::i::Queue::u8());

static mut URC_READY: bool = false;
static mut SOCKET_SET: Option<SocketSet<consts::U6, consts::U2048>> = None;

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

    static mut RES_QUEUE: ResQueue<consts::U256, consts::U5> = Queue(heapless::i::Queue::u8());
    static mut URC_QUEUE: UrcQueue<consts::U256, consts::U10> = Queue(heapless::i::Queue::u8());
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

    unsafe {
        SOCKET_SET = Some(SocketSet::new());
    }

    let mut gsm = GsmClient::<_, _, consts::U6, consts::U2048>::new(
        cell_client,
        SysTimer::new(),
        Config::new(""),
    );

    let socket_set: &'static mut _ = unsafe {
        SOCKET_SET.as_mut().unwrap_or_else(|| {
            panic!("Failed to get the static socket_set");
        })
    };

    gsm.set_socket_storage(socket_set);

    let (p, c) = unsafe { Q.split() };

    let thing_name = "test_ublox_cellular";

    // Connect to AWS IoT
    let mut mqtt_event = MqttEvent::new(
        c,
        SysTimer::new(),
        MqttOptions::new(thing_name, "broker.hivemq.com".into(), 1883),
    );

    let mut mqtt_client = MqttClient::new(p, thing_name);

    // Launch reading thread
    thread::Builder::new()
        .spawn(move || loop {
            let mut buffer = [0; 32];
            match serial_rx.read(&mut buffer[..]) {
                Ok(0) => {}
                Ok(bytes_read) => {
                    ingress.write(&buffer[0..bytes_read]);
                    ingress.digest();
                    ingress.digest();
                }
                Err(e) => match e.kind() {
                    io::ErrorKind::Interrupted => {}
                    _ => {
                        // log::error!("Serial reading thread error while reading: {}", e);
                    }
                },
            }
        })
        .unwrap();

    let apn_info = APNInfo {
        apn: Apn::Given(heapless::String::from("em")),
        ..APNInfo::default()
    };

    let mut cnt = 1;

    // Subscribe @ http://www.hivemq.com/demos/websocket-client/
    mqtt_client
        .subscribe(
            heapless::Vec::from_slice(&[
                SubscribeTopic {
                    topic_path: heapless::String::from("mqttrust/tester/subscriber"),
                    qos: QoS::AtLeastOnce,
                },
                SubscribeTopic {
                    topic_path: heapless::String::from("mqttrust/tester/subscriber2"),
                    qos: QoS::AtLeastOnce,
                },
            ])
            .unwrap(),
        )
        .expect("Failed to subscribe!");

    thread::Builder::new()
        .name("eventloop".to_string())
        .spawn(move || {
            let mut cnt = 0;
            loop {
                // Publish @ http://www.hivemq.com/demos/websocket-client/
                mqtt_client
                    .publish(
                        heapless::String::from("mqttrust/tester/publisher"),
                        heapless::Vec::from_slice(
                            format!(
                                "{{\"key\": \"Hello World from Ublox Cellular - {}!\"}}",
                                cnt
                            )
                            .as_bytes(),
                        )
                        .unwrap(),
                        QoS::AtLeastOnce,
                    )
                    .expect("Failed to publish!");
                cnt += 1;
                thread::sleep(Duration::from_millis(5000));
            }
        })
        .unwrap();

    loop {
        match gsm.data_service(ProfileId(0), ContextId(2), &apn_info) {
            Err(nb::Error::WouldBlock) => {}
            Err(nb::Error::Other(_e)) => {
                // defmt::error!("Data Service error! {:?}", e);
            }
            Ok(data) => {
                if mqtt_event.connect(&data).is_err() {
                    continue;
                }

                match mqtt_event.yield_event(&data) {
                    Ok(Notification::Publish(_publish)) => {
                        // log::debug!(
                        //     "[{}, {:?}]: {:?}",
                        //     _publish.topic_name,
                        //     _publish.qospid,
                        //     String::from_utf8(_publish.payload).unwrap()
                        // );
                    }
                    _ => {
                        // log::debug!("{:?}", n);
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
}
