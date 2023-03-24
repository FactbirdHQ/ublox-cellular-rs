extern crate alloc;

mod file_handler;

use serialport;
use std::io;
use std::thread;

use file_handler::FileHandler;

use ublox_cellular::gprs::APNInfo;
use ublox_cellular::prelude::*;
use ublox_cellular::sockets::Ipv4Addr;
use ublox_cellular::{error::Error as GSMError, Config, GsmClient};

use atat::blocking::AtatClient;
use embedded_hal::digital::v2::OutputPin;
use linux_embedded_hal::Pin;
use mqttrust::{MqttClient, MqttEvent, MqttOptions, Notification, Request};

use rustot::{
    jobs::{is_job_message, IotJobsData, JobAgent, JobDetails, JobStatus},
    ota::ota::{is_ota_message, OtaAgent, OtaConfig},
};

use heapless::{consts, spsc::Queue, ArrayLength};

use common::{serial::Serial, timer::SysTimer};
use std::time::Duration;

fn attach_gprs<C, RST, DTR>(gsm: &GsmClient<C, RST, DTR>) -> Result<(), GSMError>
where
    C: AtatClient,
    RST: OutputPin,
    DTR: OutputPin,
{
    gsm.init(true)?;

    // Load certificates
    gsm.import_root_ca(
        0,
        "Verisign",
        include_bytes!("../secrets_mini_2/Verisign.pem"),
    )?;
    gsm.import_certificate(
        0,
        "cert",
        include_bytes!("../secrets_mini_2/certificate.pem.crt"),
    )?;
    gsm.import_private_key(
        0,
        "key",
        include_bytes!("../secrets_mini_2/private.pem.key"),
        None,
    )?;

    gsm.begin("").unwrap();
    gsm.attach_gprs(APNInfo::new("em")).unwrap();
    Ok(())
}

static mut Q: Queue<Request, consts::U10> = Queue(heapless::i::Queue::new());

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

    static mut RES_QUEUE: atat::ResQueue<AtatRxBufLen> = Queue(heapless::i::Queue::u8());
    static mut URC_QUEUE: atat::UrcQueue<AtatRxBufLen> = Queue(heapless::i::Queue::u8());
    static mut COM_QUEUE: atat::ComQueue = Queue(heapless::i::Queue::u8());
    let (res_p, res_c) = unsafe { RES_QUEUE.split() };
    let (urc_p, urc_c) = unsafe { URC_QUEUE.split() };
    let (com_p, com_c) = unsafe { COM_QUEUE.split() };

    let at_config = atat::Config::new(atat::Mode::Timeout);
    let mut ingress = atat::IngressManager::with_custom_urc_matcher(
        res_p,
        urc_p,
        com_c,
        at_config,
        Some(NvicUrcMatcher::new()),
    );
    let cell_client = atat::Client::new(
        Serial(serial_tx),
        res_c,
        urc_c,
        com_p,
        SysTimer::new(),
        at_config,
    );

    let gsm = GsmClient::<_, Pin, Pin>::new(cell_client, Config::new());

    let (p, c) = unsafe { Q.split() };

    let thing_name = heapless::String::<heapless::consts::U32>::from("test_mini_2");

    // Connect to AWS IoT
    let mut mqtt_eventloop = MqttEvent::new(
        c,
        SysTimer::new(),
        MqttOptions::new(thing_name.as_str(), Ipv4Addr::new(52, 208, 158, 107), 8883)
            .set_max_packet_size(2048),
    );

    let mqtt_client = MqttClient::new(p, thing_name);

    let file_handler = FileHandler::new();
    let mut job_agent = JobAgent::new();
    let mut ota_agent = OtaAgent::new(
        file_handler,
        SysTimer::new(),
        OtaConfig::default().set_block_size(512),
    );

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
        loop {
            match mqtt_eventloop.connect(&gsm) {
                Ok(_) => {
                    break;
                }
                Err(nb::Error::Other(_e)) => panic!("Failed to connect to MQTT"),
                Err(nb::Error::WouldBlock) => {}
            }
            if unsafe { URC_READY } {
                gsm.spin().unwrap();
                unsafe { URC_READY = false };
            }
            thread::sleep(Duration::from_millis(100));
        }

        job_agent.subscribe_to_jobs(&mqtt_client).unwrap();

        job_agent
            .describe_job_execution(&mqtt_client, "$next", None, None)
            .unwrap();

        loop {
            if unsafe { URC_READY } {
                log::info!("Spinning from URC_RDY");
                gsm.spin().unwrap();
                unsafe { URC_READY = false };
            }

            ota_agent.request_timer_irq(&mqtt_client);

            match mqtt_eventloop.yield_event(&gsm) {
                Ok(Notification::Publish(publish)) => {
                    if is_job_message(&publish.topic_name) {
                        match job_agent.handle_message(&mqtt_client, &publish) {
                            Ok(None) => {}
                            Ok(Some(job)) => {
                                log::debug!("Accepted a new JOB! {:?}", job);
                                match job.details {
                                    JobDetails::OtaJob(otajob) => {
                                        ota_agent.process_ota_job(&mqtt_client, otajob).unwrap()
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "[{}, {:?}]: {:?}",
                                    publish.topic_name,
                                    publish.qospid,
                                    e
                                );
                            }
                        }
                    } else if is_ota_message(&publish.topic_name) {
                        match ota_agent.handle_message(&mqtt_client, &publish) {
                            Ok(progress) => {
                                log::info!("OTA Progress: {}%", progress);
                                if progress == 100 {
                                    job_agent
                                        .update_job_execution(&mqtt_client, JobStatus::Succeeded)
                                        .unwrap();
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "[{}, {:?}]: {:?}",
                                    publish.topic_name,
                                    publish.qospid,
                                    e
                                );
                            }
                        }
                    } else {
                        log::info!("Got some other incoming message {:?}", publish);
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
