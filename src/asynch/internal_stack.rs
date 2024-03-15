// pub mod ublox_stack;

use core::mem::MaybeUninit;

use atat::{asynch::Client, AtatIngress};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embedded_io_async::{Read, Write};

use crate::{command::Urc, config::CellularConfig};

pub use super::resources::UbxResources as Resources;

use super::{
    control::Control,
    runner::{Runner, URC_SUBSCRIBERS},
    state, AtHandle,
};

pub fn new_internal<
    'a,
    R: Read,
    W: Write,
    C: CellularConfig<'a>,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
>(
    reader: R,
    writer: W,
    resources: &'a mut Resources<W, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>,
    config: C,
) -> (
    state::Device<'a, Client<'a, W, INGRESS_BUF_SIZE>, URC_CAPACITY>,
    Control<'a, Client<'a, W, INGRESS_BUF_SIZE>>,
    InternalRunner<'a, R, W, C, INGRESS_BUF_SIZE, URC_CAPACITY>,
) {
    // safety: this is a self-referential struct, however:
    // - it can't move while the `'a` borrow is active.
    // - when the borrow ends, the dangling references inside the MaybeUninit will never be used again.
    let at_client_uninit: *mut MaybeUninit<Mutex<NoopRawMutex, Client<'a, W, INGRESS_BUF_SIZE>>> =
        (&mut resources.at_client
            as *mut MaybeUninit<Mutex<NoopRawMutex, Client<'static, W, INGRESS_BUF_SIZE>>>)
            .cast();

    unsafe { &mut *at_client_uninit }.write(Mutex::new(Client::new(
        writer,
        &resources.res_slot,
        &mut resources.cmd_buf,
        atat::Config::default(),
    )));

    let at_client = unsafe { (&*at_client_uninit).assume_init_ref() };

    let (ch_runner, net_device) = state::new(
        &mut resources.ch,
        AtHandle(at_client),
        resources.urc_channel.subscribe().unwrap(),
    );

    let control = Control::new(ch_runner.state_runner(), AtHandle(at_client));

    let runner = Runner::new(
        ch_runner,
        AtHandle(at_client),
        config,
        resources.urc_channel.subscribe().unwrap(),
    );

    let ingress = atat::Ingress::new(
        atat::AtDigester::<Urc>::new(),
        &mut resources.ingress_buf,
        &resources.res_slot,
        &resources.urc_channel,
    );

    let runner = InternalRunner {
        cellular_runner: runner,
        ingress,
        reader,
    };

    (net_device, control, runner)
}

pub struct InternalRunner<
    'a,
    R: Read,
    W: Write,
    C: CellularConfig<'a>,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub cellular_runner: Runner<'a, Client<'a, W, INGRESS_BUF_SIZE>, C, URC_CAPACITY>,
    pub ingress: atat::Ingress<
        'a,
        atat::AtDigester<Urc>,
        Urc,
        INGRESS_BUF_SIZE,
        URC_CAPACITY,
        URC_SUBSCRIBERS,
    >,
    pub reader: R,
}

impl<
        'a,
        R: Read,
        W: Write,
        C: CellularConfig<'a>,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
    > InternalRunner<'a, R, W, C, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub async fn run(&mut self) -> ! {
        embassy_futures::join::join(
            self.ingress.read_from(&mut self.reader),
            self.cellular_runner.run(),
        )
        .await;
        core::unreachable!()
    }
}
