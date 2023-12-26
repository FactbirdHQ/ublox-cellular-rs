#![no_std]
#![no_main]
#![allow(stable_features)]
// #![feature(type_alias_impl_trait)]

use atat::asynch::Client;
use atat::ResponseSlot;
use atat::UrcChannel;
use core::cell::RefCell;
use cortex_m_rt::entry;
use defmt::*;
use embassy_executor::{Executor, Spawner};
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, OutputOpenDrain, Pin, Pull, Speed};
use embassy_stm32::peripherals::UART8;
use embassy_stm32::rcc::VoltageScale;
use embassy_stm32::time::{khz, mhz};
use embassy_stm32::usart::{BufferedUart, BufferedUartRx, BufferedUartTx};
use embassy_stm32::{bind_interrupts, interrupt, peripherals, usart, Config};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

// use embedded_hal::digital::{ErrorType, InputPin, OutputPin};

use ublox_cellular;
use ublox_cellular::config::{CellularConfig, ReverseOutputPin};

use atat::asynch::AtatClient;
use atat::{AtDigester, AtatIngress, DefaultDigester, Ingress, Parser};
use ublox_cellular::asynch::runner::Runner;
use ublox_cellular::asynch::state::{LinkState, OperationState};
use ublox_cellular::asynch::State;
use ublox_cellular::command;
use ublox_cellular::command::{Urc, AT};

bind_interrupts!(struct Irqs {
    UART8 => embassy_stm32::usart::BufferedInterruptHandler<peripherals::UART8>;
});

const INGRESS_BUF_SIZE: usize = 1024;
const URC_CAPACITY: usize = 2;
const URC_SUBSCRIBERS: usize = 2;

struct MyCelullarConfig {
    reset_pin: Option<Output<'static, AnyPin>>,
    // reset_pin: Option<NoPin>,
    power_pin: Option<ReverseOutputPin<Output<'static, AnyPin>>>,
    // power_pin: Option<NoPin>,
    vint_pin: Option<Input<'static, AnyPin>>,
    // vint_pin: Option<NoPin>
}

impl CellularConfig for MyCelullarConfig {
    type ResetPin = Output<'static, AnyPin>;
    // type ResetPin = NoPin;
    type PowerPin = ReverseOutputPin<Output<'static, AnyPin>>;
    // type PowerPin = NoPin;
    type VintPin = Input<'static, AnyPin>;
    // type VintPin = NoPin;

    const FLOW_CONTROL: bool = false;
    const HEX_MODE: bool = true;
    fn reset_pin(&mut self) -> Option<&mut Self::ResetPin> {
        info!("reset_pin");
        return self.reset_pin.as_mut();
    }
    fn power_pin(&mut self) -> Option<&mut Self::PowerPin> {
        info!("power_pin");
        return self.power_pin.as_mut();
    }
    fn vint_pin(&mut self) -> Option<&mut Self::VintPin> {
        info!("vint_pin = {}", self.vint_pin.as_mut()?.is_high());
        return self.vint_pin.as_mut();
    }
}

#[embassy_executor::task]
async fn main_task(spawner: Spawner) {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.supply_config = SupplyConfig::DirectSMPS;
        config.rcc.hsi = Some(HSIPrescaler::DIV1);
        config.rcc.csi = true;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL50,
            divp: Some(PllDiv::DIV2),
            divq: Some(PllDiv::DIV8), // 100mhz
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P; // 400 Mhz
        config.rcc.ahb_pre = AHBPrescaler::DIV2; // 200 Mhz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb3_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.voltage_scale = VoltageScale::Scale1;
    }
    let p = embassy_stm32::init(config);

    let led1_pin = p.PI12.degrade();
    let led2_pin = p.PI13.degrade();
    let led3_pin = p.PI14.degrade();

    static tx_buf: StaticCell<[u8; 16]> = StaticCell::new();
    static rx_buf: StaticCell<[u8; 16]> = StaticCell::new();

    let (tx_pin, rx_pin, uart) = (p.PJ8, p.PJ9, p.UART8);
    let mut uart_config = embassy_stm32::usart::Config::default();
    {
        uart_config.baudrate = 115200;
        // uart_config.baudrate = 9600;
        uart_config.parity = embassy_stm32::usart::Parity::ParityNone;
        uart_config.stop_bits = embassy_stm32::usart::StopBits::STOP1;
        uart_config.data_bits = embassy_stm32::usart::DataBits::DataBits8;
    }

    let uart = BufferedUart::new(
        uart,
        Irqs,
        rx_pin,
        tx_pin,
        tx_buf.init([0u8; 16]),
        rx_buf.init([0u8; 16]),
        uart_config,
    );
    let (writer, reader) = uart.unwrap().split();
    // let power = Output::new(p.PJ4, Level::High, Speed::VeryHigh).degrade();
    // let reset = Output::new(p.PF8, Level::High, Speed::VeryHigh).degrade();
    let celullar_config = MyCelullarConfig {
        reset_pin: Some(Output::new(p.PF8, Level::High, Speed::Low).degrade()),
        power_pin: Some(ReverseOutputPin(
            Output::new(p.PJ4, Level::High, Speed::Low).degrade(),
        )),
        // reset_pin: Some(OutputOpenDrain::new(p.PF8, Level::High, Speed::Low, Pull::None).degrade()),
        // power_pin: Some(OutputOpenDrain::new(p.PJ4, Level::High, Speed::Low, Pull::None).degrade()),
        // power_pin: None,
        vint_pin: Some(Input::new(p.PJ3, Pull::Down).degrade()),
    };

    static RES_SLOT: ResponseSlot<INGRESS_BUF_SIZE> = ResponseSlot::new();
    static URC_CHANNEL: UrcChannel<command::Urc, URC_CAPACITY, URC_SUBSCRIBERS> = UrcChannel::new();
    let ingress = Ingress::new(
        DefaultDigester::<command::Urc>::default(),
        &RES_SLOT,
        &URC_CHANNEL,
    );
    static buf: StaticCell<[u8; INGRESS_BUF_SIZE]> = StaticCell::new();
    let mut client = Client::new(
        writer,
        &RES_SLOT,
        buf.init([0; INGRESS_BUF_SIZE]),
        atat::Config::default(),
    );

    spawner.spawn(ingress_task(ingress, reader)).unwrap();

    static state: StaticCell<State<Client<BufferedUartTx<UART8>, INGRESS_BUF_SIZE>>> =
        StaticCell::new();
    let (device, mut control, mut runner) = ublox_cellular::asynch::new(
        state.init(State::new(client)),
        &URC_CHANNEL,
        celullar_config,
    )
    .await;
    // defmt::info!("{:?}", runner.init().await);
    // control.set_desired_state(PowerState::Connected).await;

    defmt::unwrap!(spawner.spawn(cellular_task(runner)));
    Timer::after(Duration::from_millis(1000)).await;
    loop {
        control.set_desired_state(OperationState::Alive).await;
        info!("set_desired_state(PowerState::Alive)");
        while control.power_state() != OperationState::Alive {
            Timer::after(Duration::from_millis(1000)).await;
        }
        Timer::after(Duration::from_millis(1000)).await;
        control.set_desired_state(OperationState::PowerDown).await;
        info!("set_desired_state(PowerState::PowerDown)");
        while control.power_state() != OperationState::PowerDown {
            Timer::after(Duration::from_millis(1000)).await;
        }
        Timer::after(Duration::from_millis(5000)).await;
    }

    loop {
        Timer::after(Duration::from_millis(1000)).await;
    }
}

#[embassy_executor::task]
async fn ingress_task(
    mut ingress: Ingress<
        'static,
        DefaultDigester<Urc>,
        ublox_cellular::command::Urc,
        { INGRESS_BUF_SIZE },
        { URC_CAPACITY },
        { URC_SUBSCRIBERS },
    >,
    mut reader: BufferedUartRx<'static, UART8>,
) -> ! {
    ingress.read_from(&mut reader).await;
    defmt::panic!("ingress_task ended");
}

#[embassy_executor::task]
async fn cellular_task(
    runner: Runner<
        'static,
        atat::asynch::Client<'_, BufferedUartTx<'static, UART8>, { INGRESS_BUF_SIZE }>,
        MyCelullarConfig,
        { URC_CAPACITY },
    >,
) -> ! {
    runner.run().await
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("Hello World!");

    let executor = EXECUTOR.init(Executor::new());

    executor.run(|spawner| {
        unwrap!(spawner.spawn(main_task(spawner)));
    })
}
