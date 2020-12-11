extern crate alloc;

mod file_handler;

use embedded_nal::{nb, Ipv4Addr};
use std::{io, thread, time::Duration};

use file_handler::FileHandler;

use atat::{ClientBuilder, ComQueue, Queues, ResQueue, UrcQueue};
use mqttrust::{MqttClient, MqttEvent, MqttOptions, Notification, Request};
use ublox_cellular::{sockets::SocketSet, APNInfo, Apn, Config, ContextId, GsmClient, ProfileId};

use rustot::{
    jobs::{is_job_message, IotJobsData, JobAgent, JobDetails},
    ota::ota::{is_ota_message, OtaAgent, OtaConfig},
};

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

    let thing_name = "test_mini_2";

    // Connect to AWS IoT
    let mut mqtt_event = MqttEvent::new(
        c,
        SysTimer::new(),
        MqttOptions::new(thing_name, Ipv4Addr::new(52, 208, 158, 107).into(), 8883),
    );

    let mut mqtt_client = MqttClient::new(p, thing_name);

    let file_handler = FileHandler::new();
    let mut job_agent = JobAgent::new();
    let mut ota_agent = OtaAgent::new(
        file_handler,
        SysTimer::new(),
        OtaConfig::default().set_block_size(512),
    );

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
    loop {
        match gsm.data_service(ProfileId(0), ContextId(2), &apn_info) {
            Err(nb::Error::WouldBlock) => {}
            Err(nb::Error::Other(_e)) => {
                // defmt::error!("Data Service error! {:?}", e);
            }
            Ok(data) => {
                match mqtt_event.connect(&data) {
                    Err(e) => {
                        continue;
                    }
                    Ok(new_session) => {
                        if new_session {
                            job_agent.subscribe_to_jobs(&mut mqtt_client).unwrap();

                            job_agent
                                .describe_job_execution(&mut mqtt_client, "$next", None, None)
                                .unwrap();
                        }
                    }
                }

                match mqtt_event.yield_event(&data) {
                    Ok(Notification::Publish(publish)) => {
                        if is_job_message(&publish.topic_name) {
                            match job_agent.handle_message(&mut mqtt_client, &publish) {
                                Ok(None) => {}
                                Ok(Some(job)) => {
                                    // log::debug!("Accepted a new JOB! {:?}", job);
                                    match job.details {
                                        JobDetails::OtaJob(otajob) => ota_agent
                                            .process_ota_job(&mut mqtt_client, otajob)
                                            .unwrap(),
                                        _ => {}
                                    }
                                }
                                Err(e) => {
                                    // log::error!(
                                    //     "[{}, {:?}]: {:?}",
                                    //     publish.topic_name,
                                    //     publish.qospid,
                                    //     e
                                    // );
                                }
                            }
                        } else if is_ota_message(&publish.topic_name) {
                            match ota_agent.handle_message(&mut mqtt_client, &mut job_agent, &mut publish) {
                                Ok(()) => {
                                    // log::info!("OTA Finished successfully");
                                }
                                Err(e) => {
                                    // log::error!(
                                    //     "[{}, {:?}]: {:?}",
                                    //     publish.topic_name,
                                    //     publish.qospid,
                                    //     e
                                    // );
                                }
                            }
                        } else {
                            // log::info!("Got some other incoming message {:?}", publish);
                        }
                    }
                    _ => {
                        // log::debug!("{:?}", n);
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
