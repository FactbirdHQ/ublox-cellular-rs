use atat::{ClientBuilder, ComQueue, Queues};
use serialport;
use std::io;
use std::thread;

use ublox_cellular::prelude::*;
use ublox_cellular::APNInfo;
//use ublox_cellular::sockets::{Ipv4Addr, Mode, SocketAddrV4, SocketSet, Socket};
use atat::AtatClient;
use ublox_cellular::sockets::SocketSet;
use ublox_cellular::{error::Error as GSMError, Config, GsmClient};

use atat::bbqueue::BBBuffer;
use heapless::{self, spsc::Queue};

use common::{gpio::ExtPin, serial::Serial, timer::SysTimer};
use std::time::Duration;

const RX_BUF_LEN: usize = 256;
const RES_CAPACITY: usize = 5;
const URC_CAPACITY: usize = 10;
const TIMER_HZ: u32 = 1000;
const MAX_SOCKET_COUNT: usize = 6;
const SOCKET_RING_BUFFER_LEN: usize = 1024;

static mut SOCKET_SET: Option<SocketSet<TIMER_HZ, MAX_SOCKET_COUNT, SOCKET_RING_BUFFER_LEN>> = None;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    // Open serial
    let serial_tx = serialport::new("/dev/ttyUSB0", 115_200)
        .timeout(Duration::from_millis(5000))
        .open()
        .expect("Could not open serial port");

    let mut serial_rx = serial_tx.try_clone().expect("Failed to clone serial port");

    static mut RES_QUEUE: BBBuffer<RES_CAPACITY> = BBBuffer::new();
    static mut URC_QUEUE: BBBuffer<URC_CAPACITY> = BBBuffer::new();
    static mut COM_QUEUE: atat::ComQueue = Queue::new();

    let queues = Queues {
        res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
        urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
        com_queue: unsafe { COM_QUEUE.split() },
    };

    let (cell_client, mut ingress) =
        ClientBuilder::<_, _, _, _, TIMER_HZ, RX_BUF_LEN, RES_CAPACITY, URC_CAPACITY>::new(
            Serial(serial_tx),
            SysTimer::new(),
            atat::Config::new(atat::Mode::Timeout),
        )
        .build(queues);

    unsafe {
        SOCKET_SET = Some(SocketSet::new());
    }

    // let gsm = GsmClient::<_, Pin, Pin, _, _>::new(
    //     cell_client,
    //     unsafe { SOCKET_SET.as_mut().unwrap() },
    //     Config::new(APNInfo::new("em"), ""),
    // );

    //let gsm = GsmClient::<_, Pin, Pin, _, _>::new(cell_client, Config::new(APNInfo::new("em"), ""));
    let mut modem = GsmClient::<
        _,
        _,
        ExtPin,
        ExtPin,
        ExtPin,
        ExtPin,
        TIMER_HZ,
        MAX_SOCKET_COUNT,
        SOCKET_RING_BUFFER_LEN,
    >::new(
        cell_client,
        SysTimer::new(),
        Config::new("").with_apn_info(APNInfo::new("em")),
    );

    modem.set_socket_storage(unsafe { SOCKET_SET.as_mut().unwrap() });

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


    modem.initialize().unwrap();

    loop {
        if let Err(e) = modem.spin() {
            log::error!("gsm spin: {:?}", e);
        }
    }

    //if attach_gprs(&gsm).is_ok() {
    // let mut socket = {
    //     let soc = <GsmClient<_, _, _, _, _> as TcpStack>::open(&gsm, Mode::Blocking)
    //         .expect("Cannot open socket!");

    //     gsm.connect(
    //         soc,
    //         // Connect to echo.u-blox.com:7
    //         SocketAddrV4::new(Ipv4Addr::new(195, 34, 89, 241), 7).into(),
    //     )
    //     .expect("Failed to connect to remote!")
    // };

    // let mut cnt = 1;
    // loop {
    //     thread::sleep(Duration::from_millis(5000));
    //     let mut buf = [0u8; 256];
    //     let read = <GsmClient<_, _, _, _, _> as TcpStack>::read(&gsm, &mut socket, &mut buf)
    //         .expect("Failed to read from socket!");
    //     if read > 0 {
    //         log::info!("Read {:?} bytes from socket layer!  - {:?}", read, unsafe {
    //             core::str::from_utf8_unchecked(&buf[..read])
    //         });
    //     }
    //     let _wrote = <GsmClient<_, _, _, _, _> as TcpStack>::write(
    //         &gsm,
    //         &mut socket,
    //         format!("Whatup {}", cnt).as_bytes(),
    //     )
    //     .expect("Failed to write to socket!");
    //     log::info!(
    //         "Writing {:?} bytes to socket layer! - {:?}",
    //         _wrote,
    //         format!("Whatup {}", cnt)
    //     );
    //     cnt += 1;
    // }
    //}
}
