use atat::{ClientBuilder, ComQueue, Queues, ResQueue, UrcQueue};
use std::{thread, io};
use ublox_cellular::{
    sockets::{
        udp::{Ipv4Addr, SocketAddrV4},
        SocketSet,
    },
    APNInfo, Apn, Config, ContextId, GsmClient, ProfileId,
};

use common::{timer::SysTimer, serial::{Serial, serialport}};
use embedded_nal::nb;
use embedded_nal::TcpClient;
use heapless::{self, consts, spsc::Queue};
use std::time::Duration;

static mut SOCKET_SET: Option<SocketSet<consts::U6, consts::U2048>> = None;

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

    let (cell_client, mut ingress) = ClientBuilder::<_, _, atat::NoopUrcMatcher, _, _, _, _>::new(
        Serial(serial_tx),
        SysTimer::new(),
        atat::Config::new(atat::Mode::Timeout),
    )
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
                let mut socket = data.socket().expect("Cannot open socket!");

                data.connect(
                    &mut socket,
                    // Connect to echo.u-blox.com:7
                    SocketAddrV4::new(Ipv4Addr::new(195, 34, 89, 241), 7).into(),
                )
                .expect("Failed to connect to remote!");

                thread::sleep(Duration::from_millis(5000));
                let mut buf = [0u8; 256];
                let read = nb::block!(data.receive(&mut socket, &mut buf))
                    .expect("Failed to read from socket!");

                if read > 0 {
                    // YAY
                }

                let _wrote =
                    nb::block!(data.send(&mut socket, format!("Whatup {}", cnt).as_bytes(),))
                        .expect("Failed to write to socket!");
                cnt += 1;
            }
        }
    }
}
