extern crate atat;
extern crate env_logger;
extern crate nb;

mod common;

use serialport;
use std::sync::{Arc, Mutex};
use std::thread;

use ublox_cellular::gprs::APNInfo;
use ublox_cellular::prelude::*;
use ublox_cellular::soc::{Ipv4Addr, Mode, SocketAddrV4};
use ublox_cellular::{error::Error as GSMError, GSMClient, GSMConfig};

use atat::AtatClient;
use embedded_hal::digital::v2::OutputPin;

use linux_embedded_hal::Pin;

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

    let serial_irq = thread::Builder::new()
        .name("serial_irq".to_string())
        .spawn(move || loop {
            thread::sleep(Duration::from_millis(1));
            if parser.handle_irq() {
                parser.parse_at();
            }
        })
        .unwrap();

    if attach_gprs(&gsm).is_ok() {
        let mut socket = {
            let soc = gsm.open(Mode::Blocking).expect("Cannot open socket!");

            gsm.connect(
                soc,
                SocketAddrV4::new(Ipv4Addr::new(195, 34, 89, 241), 7).into(),
            )
            .expect("Failed to connect to remote!")
        };
        let urc_gsm = Arc::new(Mutex::new(gsm));
        let data_gsm = urc_gsm.clone();
        thread::Builder::new()
            .spawn(move || loop {
                thread::sleep(Duration::from_millis(10));
                urc_gsm.lock().unwrap().handle_urc();
            })
            .expect("Failed to create URC Handler thread");

        let mut cnt = 1;
        loop {
            thread::sleep(Duration::from_millis(5000));
            let mut buf = [0u8; 256];
            {
                let gsm = data_gsm.lock().unwrap();
                let read = gsm
                    .read(&mut socket, &mut buf)
                    .expect("Failed to read from socket!");
                if read > 0 {
                    log::info!(
                        "Read {:?} bytes from socket layer!  - {:?}\r",
                        read,
                        unsafe { core::str::from_utf8_unchecked(&buf[..read]) }
                    );
                }
            }

            {
                let gsm = data_gsm.lock().unwrap();
                let wrote = gsm
                    .write(&mut socket, format!("Whatup {}", cnt).as_bytes())
                    .expect("Failed to write to socket!");
                log::info!(
                    "Writing {:?} bytes to socket layer! - {:?}\r",
                    wrote,
                    format!("Whatup {}", cnt)
                );
            }
            cnt += 1;
        }
    }

    // wait for all the threads to join back (Will never happen in this example)
    serial_irq.join().unwrap();
}
