#![no_std]
#![no_main]
#![allow(stable_features)]
#![feature(type_alias_impl_trait)]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_rp::gpio::Input;

use embassy_rp::gpio::OutputOpenDrain;
use embassy_rp::uart::BufferedUart;
use embassy_rp::uart::BufferedUartRx;
use embassy_rp::uart::BufferedUartTx;
use embassy_rp::{bind_interrupts, peripherals::UART0, uart::BufferedInterruptHandler};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use ublox_cellular::asynch::InternalRunner;
use ublox_cellular::asynch::Resources;
use {defmt_rtt as _, panic_probe as _};

use ublox_cellular::config::{Apn, CellularConfig};

use ublox_cellular::asynch::state::OperationState;

bind_interrupts!(struct Irqs {
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

const CMD_BUF_SIZE: usize = 128;
const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 2;

struct MyCelullarConfig {
    reset_pin: Option<OutputOpenDrain<'static>>,
    power_pin: Option<OutputOpenDrain<'static>>,
    vint_pin: Option<Input<'static>>,
}

impl<'a> CellularConfig<'a> for MyCelullarConfig {
    type ResetPin = OutputOpenDrain<'static>;
    type PowerPin = OutputOpenDrain<'static>;
    type VintPin = Input<'static>;

    const FLOW_CONTROL: bool = true;
    const HEX_MODE: bool = true;
    const APN: Apn<'a> = Apn::Given {
        name: "em",
        username: None,
        password: None,
    };
    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        info!("reset_pin");
        self.reset_pin.as_mut()
    }
    fn power_pin(&mut self) -> Option<&mut Self::PowerPin> {
        info!("power_pin");
        self.power_pin.as_mut()
    }
    fn vint_pin(&mut self) -> Option<&mut Self::VintPin> {
        info!("vint_pin = {}", self.vint_pin.as_mut()?.is_high());
        self.vint_pin.as_mut()
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = {
        let config =
            embassy_rp::config::Config::new(embassy_rp::clocks::ClockConfig::crystal(12_000_000));
        embassy_rp::init(config)
    };

    static TX_BUF: StaticCell<[u8; 16]> = StaticCell::new();
    static RX_BUF: StaticCell<[u8; 16]> = StaticCell::new();

    let cell_uart = BufferedUart::new_with_rtscts(
        p.UART0,
        Irqs,
        p.PIN_0,
        p.PIN_1,
        p.PIN_3,
        p.PIN_2,
        TX_BUF.init([0; 16]),
        RX_BUF.init([0; 16]),
        embassy_rp::uart::Config::default(),
    );

    let (uart_rx, uart_tx) = cell_uart.split();
    let cell_nrst = gpio::OutputOpenDrain::new(p.PIN_4, gpio::Level::High);
    let cell_pwr = gpio::OutputOpenDrain::new(p.PIN_5, gpio::Level::High);
    let cell_vint = gpio::Input::new(p.PIN_6, gpio::Pull::None);

    static RESOURCES: StaticCell<
        Resources<BufferedUartTx<UART0>, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
    > = StaticCell::new();

    let (_net_device, mut control, runner) = ublox_cellular::asynch::new_internal(
        uart_rx,
        uart_tx,
        RESOURCES.init(Resources::new()),
        MyCelullarConfig {
            reset_pin: Some(cell_nrst),
            power_pin: Some(cell_pwr),
            vint_pin: Some(cell_vint),
        },
    );

    // defmt::info!("{:?}", runner.init().await);
    // control.set_desired_state(PowerState::Connected).await;
    // control
    //     .send(&crate::command::network_service::SetOperatorSelection {
    //         mode: crate::command::network_service::types::OperatorSelectionMode::Automatic,
    //         format: Some(0),
    //     })
    //     .await;

    defmt::unwrap!(spawner.spawn(cell_task(runner)));

    Timer::after(Duration::from_millis(1000)).await;
    loop {
        control
            .set_desired_state(OperationState::DataEstablished)
            .await;
        info!("set_desired_state(PowerState::Alive)");
        while control.power_state() != OperationState::DataEstablished {
            Timer::after(Duration::from_millis(1000)).await;
        }
        Timer::after(Duration::from_millis(10000)).await;

        loop {
            Timer::after(Duration::from_millis(1000)).await;
            let operator = control.get_operator().await;
            info!("{}", operator);
            let signal_quality = control.get_signal_quality().await;
            info!("{}", signal_quality);
            if signal_quality.is_err() {
                let desired_state = control.desired_state();
                control.set_desired_state(desired_state).await
            }
            if let Ok(sq) = signal_quality {
                if let Ok(op) = operator {
                    if op.oper.is_none() {
                        continue;
                    }
                }
                if sq.rxlev > 0 && sq.rsrp != 255 {
                    break;
                }
            }
        }
        let dns = control
            .send(&ublox_cellular::command::dns::ResolveNameIp {
                resolution_type:
                    ublox_cellular::command::dns::types::ResolutionType::DomainNameToIp,
                ip_domain_string: "www.google.com",
            })
            .await;
        info!("dns: {:?}", dns);
        Timer::after(Duration::from_millis(10000)).await;
        control.set_desired_state(OperationState::PowerDown).await;
        info!("set_desired_state(PowerState::PowerDown)");
        while control.power_state() != OperationState::PowerDown {
            Timer::after(Duration::from_millis(1000)).await;
        }

        Timer::after(Duration::from_millis(5000)).await;
    }
}

// #[embassy_executor::task]
// async fn net_task(stack: &'static Stack<embassy_net_ppp::Device<'static>>) -> ! {
//     stack.run().await
// }

#[embassy_executor::task]
async fn cell_task(
    mut runner: InternalRunner<
        'static,
        BufferedUartRx<'static, UART0>,
        BufferedUartTx<'static, UART0>,
        MyCelullarConfig,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
    >,
) -> ! {
    runner.run().await
}
