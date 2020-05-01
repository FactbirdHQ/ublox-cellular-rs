use serialport;
use std::io;
use std::thread;

use ublox_cellular::gprs::APNInfo;
use ublox_cellular::prelude::*;
use ublox_cellular::sockets::{Ipv4Addr, Mode, SocketAddrV4};
use ublox_cellular::{error::Error as GSMError, Config, GsmClient};

use atat::AtatClient;
use embedded_hal::digital::v2::OutputPin;

use linux_embedded_hal::Pin;

use common::{serial::Serial, timer::SysTimer};
use std::time::Duration;

fn attach_gprs<C, RST, DTR>(gsm: &GsmClient<C, RST, DTR>) -> Result<(), GSMError>
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
        timeout: Duration::from_millis(5000),
    };

    // Open serial port
    let serial_tx = serialport::open_with_settings("/dev/ttyUSB0", &settings)
        .expect("Could not open serial port");
    let mut serial_rx = serial_tx.try_clone().expect("Failed to clone serial port");

    let (cell_client, mut ingress) = atat::ClientBuilder::<_, _, atat::NoopUrcMatcher>::new(
        Serial(serial_tx),
        SysTimer::new(),
        atat::Config::new(atat::Mode::Timeout),
    )
    .build();

    let gsm = GsmClient::<_, Pin, Pin>::new(cell_client, Config::new());

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
                        log::error!("Serial reading thread error while reading: {}", e);
                    }
                },
            }
        })
        .unwrap();

    if attach_gprs(&gsm).is_ok() {
        let mut socket = {
            let soc = <GsmClient<_, _, _> as TcpStack>::open(&gsm, Mode::Blocking)
                .expect("Cannot open socket!");

            gsm.connect(
                soc,
                // Connect to echo.u-blox.com:7
                SocketAddrV4::new(Ipv4Addr::new(195, 34, 89, 241), 7).into(),
            )
            .expect("Failed to connect to remote!")
        };

        let mut cnt = 1;
        loop {
            thread::sleep(Duration::from_millis(5000));
            let mut buf = [0u8; 256];
            let read = <GsmClient<_, _, _> as TcpStack>::read(&gsm, &mut socket, &mut buf)
                .expect("Failed to read from socket!");
            if read > 0 {
                log::info!("Read {:?} bytes from socket layer!  - {:?}", read, unsafe {
                    core::str::from_utf8_unchecked(&buf[..read])
                });
            }
            let _wrote = <GsmClient<_, _, _> as TcpStack>::write(
                &gsm,
                &mut socket,
                format!("Whatup {}", cnt).as_bytes(),
            )
            .expect("Failed to write to socket!");
            log::info!(
                "Writing {:?} bytes to socket layer! - {:?}",
                _wrote,
                format!("Whatup {}", cnt)
            );
            cnt += 1;
        }
    }
}
