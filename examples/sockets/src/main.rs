use std::thread;
use std::time::Duration;

use atat::bbqueue::BBBuffer;
use atat::heapless::spsc::Queue;
use common::{gpio::ExtPin, serial::Serial, timer::SysTimer};
use serialport;
use structopt::StructOpt;
use ublox_cellular::atat;
use ublox_cellular::prelude::*;
use ublox_cellular::sockets::{SocketHandle, SocketSet};
use ublox_cellular::{APNInfo, Config, GsmClient};

// TODO: better naming and explanations
const RX_BUF_LEN: usize = 256;
const RES_CAPACITY: usize = 256;
const URC_CAPACITY: usize = 256;
const TIMER_HZ: u32 = 1000;
const MAX_SOCKET_COUNT: usize = 6;
const SOCKET_RING_BUFFER_LEN: usize = 1024;

static mut SOCKET_SET: Option<SocketSet<TIMER_HZ, MAX_SOCKET_COUNT, SOCKET_RING_BUFFER_LEN>> = None;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Serial port device
    #[structopt(short, long, default_value = "/dev/ttyUSB0")]
    port: String,

    /// Serial port baudrate
    #[structopt(short, long, default_value = "115200")]
    baud: u32,
}

#[derive(Debug)]
enum NetworkError {
    SocketOpen,
    SocketConnect,
    SocketClosed,
}

fn connect<N: TcpClientStack<TcpSocket = SocketHandle> + ?Sized>(
    socket: &mut Option<SocketHandle>,
    network: &mut N,
    socket_addr: SocketAddr,
) -> Result<(), NetworkError> {
    let sock = match socket.as_mut() {
        None => {
            let sock = network.socket().map_err(|_e| NetworkError::SocketOpen)?;
            socket.get_or_insert(sock)
        }
        Some(sock) => sock,
    };

    nb::block!(network.connect(sock, socket_addr)).map_err(|_| {
        socket.take();
        NetworkError::SocketConnect
    })
}

fn is_connected<N: TcpClientStack<TcpSocket = SocketHandle> + ?Sized>(
    socket: &Option<SocketHandle>,
    network: &mut N,
) -> Result<bool, NetworkError> {
    match socket {
        Some(ref socket) => network
            .is_connected(socket)
            .map_err(|_e| NetworkError::SocketClosed),
        None => Err(NetworkError::SocketClosed),
    }
}

fn main() {
    let opt = Opt::from_args();

    // different log levels can be set using RUST_LOG env variable
    // this sets common_lib to info and all others to debug:
    // RUST_LOG=common=info,debug ./target/debug/sockets --port /dev/tty.usbserial-01028661
    // use comma separate list to add specific log levels to other modules:
    // RUST_LOG=common=info,atat=info,debug ./target/debug/sockets --port /dev/tty.usbserial-01028661
    env_logger::builder().format_timestamp_millis().init();

    let serial_tx = serialport::new(opt.port, opt.baud)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Could not open serial port");

    let mut serial_rx = serial_tx.try_clone().expect("Failed to clone serial port");

    static mut RES_QUEUE: BBBuffer<RES_CAPACITY> = BBBuffer::new();
    static mut URC_QUEUE: BBBuffer<URC_CAPACITY> = BBBuffer::new();
    static mut COM_QUEUE: atat::ComQueue = Queue::new();

    let queues = atat::Queues {
        res_queue: unsafe { RES_QUEUE.try_split_framed().unwrap() },
        urc_queue: unsafe { URC_QUEUE.try_split_framed().unwrap() },
        com_queue: unsafe { COM_QUEUE.split() },
    };

    let (cell_client, mut ingress) =
        atat::ClientBuilder::<_, _, _, _, TIMER_HZ, RX_BUF_LEN, RES_CAPACITY, URC_CAPACITY>::new(
            Serial(serial_tx),
            SysTimer::new("RX"),
            atat::Config::new(atat::Mode::Timeout),
        )
        .build(queues);

    unsafe {
        SOCKET_SET = Some(SocketSet::new());
    }

    let mut cell_client = GsmClient::<
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
        SysTimer::new("CELL"),
        Config::new("").with_apn_info(APNInfo::new("internet.tele2.ee")),
    );

    cell_client.set_socket_storage(unsafe { SOCKET_SET.as_mut().unwrap() });

    // spawn serial reading thread
    thread::Builder::new()
        .spawn(move || loop {
            ingress.digest();

            let mut buffer = [0; 32];
            match serial_rx.read(&mut buffer[..]) {
                Ok(0) => {}
                Ok(bytes_read) => {
                    //ingress.digest();
                    // log::info!(
                    //     "rx: {:?}",
                    //     std::str::from_utf8(&buffer[..bytes_read]).unwrap()
                    // );
                    ingress.write(&buffer[0..bytes_read]);
                    ingress.digest();
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::Interrupted => {}
                    _ => {
                        //log::error!("Serial reading thread error while reading: {}", e);
                    }
                },
            }
            //ingress.digest();
        })
        .unwrap();

    let mut socket: Option<SocketHandle> = None;
    let mut count = 0;

    // notice that `.data_service` must be called continuously to tick modem state machine
    loop {
        cell_client
            .data_service(&APNInfo::new("em"))
            .and_then(|mut service| {
                match is_connected(&socket, &mut service) {
                    Ok(false) => {
                        // socket is present, but not connected
                        // usually this implies that the socket is closed for writes
                        // close and recycle the socket
                        let sock = socket.take().unwrap();
                        TcpClientStack::close(&mut service, sock).expect("cannot close socket");
                    }
                    Err(_) => {
                        // socket not available, try to create and connect
                        if let Err(e) = connect(
                            &mut socket,
                            &mut service,
                            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(195, 34, 89, 241)), 7),
                        ) {
                            log::error!("cannot connect {:?}", e);
                        }
                    }
                    Ok(true) => {
                        // socket is available, and connected.
                    }
                }

                // socket can be used if connected
                socket.as_mut().and_then(|sock| {
                    if let Err(e) = nb::block!(TcpClientStack::send(
                        &mut service,
                        sock,
                        format!("Whatup {}", count).as_bytes()
                    )) {
                        log::error!("cannot send {:?}", e);
                    }

                    let mut buf = [0; 32];
                    match nb::block!(TcpClientStack::receive(&mut service, sock, &mut buf)) {
                        Ok(count) => {
                            log::info!("received {} bytes: {:?}", count, &buf[..count]);
                        }
                        Err(e) => {
                            log::error!("cannot receive {:?}", e);
                        }
                    }
                    Some(())
                });

                Ok(())
            })
            .ok();

        count += 1;
    }
}
