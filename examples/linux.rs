use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use linux_embedded_hal::Serial;
use serial::{self, core::SerialPort};

extern crate at_rs as at;
extern crate env_logger;
extern crate nb;

// Note this useful idiom: importing names from outer (for mod tests) scope.
use ublox_cellular::command::*;
use ublox_cellular::prelude::*;
use ublox_cellular::GSMClient;

use heapless::{consts::*, spsc::Queue, String};
#[allow(unused_imports)]
use log::{error, info, warn};

#[derive(Clone, Copy)]
struct MilliSeconds(u32);

trait U32Ext {
    fn s(self) -> MilliSeconds;
    fn ms(self) -> MilliSeconds;
}

impl U32Ext for u32 {
    fn s(self) -> MilliSeconds {
        MilliSeconds(self / 1000)
    }
    fn ms(self) -> MilliSeconds {
        MilliSeconds(self)
    }
}

impl From<u32> for MilliSeconds {
    fn from(ms: u32) -> Self {
        MilliSeconds(ms)
    }
}

struct Timer;

impl embedded_hal::timer::CountDown for Timer {
    type Time = MilliSeconds;
    fn start<T>(&mut self, _duration: T)
    where
        T: Into<MilliSeconds>,
    {
        // let dur = duration.into();
        // self.timeout_time = Instant::now().checked_add(Duration::from_millis(dur.0.into())).expect("");
    }

    fn wait(&mut self) -> ::nb::Result<(), void::Void> {
        // if self.timeout_time - Instant::now() < Duration::from_secs(0) {
        // Ok(())
        // } else {
        Err(nb::Error::WouldBlock)
        // }
    }
}

impl embedded_hal::timer::Cancel for Timer {
    type Error = ();
    fn cancel(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

type SerialRxBufferLen = U4096;
type ATRequestQueueLen = U5;
type ATResponseQueueLen = U5;

// static mut Test: Option<Queue<Box<dyn ATCommand<Response = ()>>, ATRequestQueueLen, u8>> = None;
static mut CELL_REQ_Q: Option<Queue<RequestType, ATRequestQueueLen, u8>> = None;
static mut CELL_RES_Q: Option<Queue<Result<ResponseType, atat::Error>, ATResponseQueueLen, u8>> =
    None;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    // Serial port settings
    let settings = serial::PortSettings {
        baud_rate: serial::Baud115200,
        char_size: serial::Bits8,
        parity: serial::ParityNone,
        stop_bits: serial::Stop1,
        flow_control: serial::FlowNone,
    };

    // Open serial port
    let mut port = serial::open("/dev/ttyACM0").expect("Could not open serial port");
    port.configure(&settings)
        .expect("Could not configure serial port");

    port.set_timeout(Duration::from_millis(2))
        .expect("Could not set serial port timeout");

    unsafe { CELL_REQ_Q = Some(Queue::u8()) };
    unsafe { CELL_RES_Q = Some(Queue::u8()) };


    let (cell_client, parser) = atat::new::<_, _, _, SerialRxBufferLen, _, _>(
        unsafe { (CELL_REQ_Q.as_mut().unwrap(), CELL_RES_Q.as_mut().unwrap()) },
        Serial(port),
        Timer,
        1000.ms(),
    );

    let mut gsm = GSMClient::new(cell_client, None, None);

    let at_parser_arc = Arc::new(Mutex::new(parser));

    let at_parser = at_parser_arc.clone();
    let serial_irq = thread::Builder::new()
        .name("serial_irq".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(1));
            if let Ok(mut at) = at_parser.lock() {
                at.handle_irq()
            }
        })
        .unwrap();

    let serial_loop = thread::Builder::new()
        .name("serial_loop".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(100));
            if let Ok(mut at) = at_parser_arc.lock() {
                at.spin()
            }
        })
        .unwrap();

    let main_loop = thread::Builder::new()
        .name("main_loop".to_string())
        .spawn(move || {
            // let networks = CELL_client.scan().unwrap();
            // networks.iter().for_each(|n| info!("{:?}", n.ssid));

            info!("Connected!");
        })
        .unwrap();

    // wait for all the threads to join back (Will never happen in this example)
    serial_irq.join().unwrap();
    serial_loop.join().unwrap();
    main_loop.join().unwrap();
}
