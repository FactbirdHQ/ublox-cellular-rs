pub mod control;
pub mod runner;
#[cfg(feature = "ublox-sockets")]
pub mod ublox_stack;

pub mod state;

use crate::{command::Urc, config::CellularConfig};
use atat::{asynch::AtatClient, AtatUrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use runner::Runner;
use state::Device;

use self::control::Control;

pub struct AtHandle<'d, AT: AtatClient>(&'d Mutex<NoopRawMutex, AT>);

impl<'d, AT: AtatClient> AtHandle<'d, AT> {
    async fn send<Cmd: atat::AtatCmd<LEN>, const LEN: usize>(
        &mut self,
        cmd: Cmd,
    ) -> Result<Cmd::Response, atat::Error> {
        self.0.lock().await.send_retry::<Cmd, LEN>(&cmd).await
    }
}

pub struct State<AT: AtatClient, const MAX_STATE_LISTENERS: usize> {
    ch: state::State<MAX_STATE_LISTENERS>,
    at_handle: Mutex<NoopRawMutex, AT>,
}

impl<AT: AtatClient, const MAX_STATE_LISTENERS: usize> State<AT, MAX_STATE_LISTENERS> {
    pub fn new(at_handle: AT) -> Self {
        Self {
            ch: state::State::new(),
            at_handle: Mutex::new(at_handle),
        }
    }
}

pub async fn new<
    'a,
    AT: AtatClient,
    SUB: AtatUrcChannel<Urc, URC_CAPACITY, 2>,
    C: CellularConfig,
    const URC_CAPACITY: usize,
    const MAX_STATE_LISTENERS: usize,
>(
    state: &'a mut State<AT, MAX_STATE_LISTENERS>,
    subscriber: &'a SUB,
    config: C,
) -> (
    Device<'a, AT, URC_CAPACITY, MAX_STATE_LISTENERS>,
    Control<'a, AT, MAX_STATE_LISTENERS>,
    Runner<'a, AT, C, URC_CAPACITY, MAX_STATE_LISTENERS>,
) {
    let (ch_runner, net_device) = state::new(
        &mut state.ch,
        AtHandle(&state.at_handle),
        subscriber.subscribe().unwrap(),
    );
    let state_ch = ch_runner.state_runner();

    let mut runner = Runner::new(
        ch_runner,
        AtHandle(&state.at_handle),
        config,
        subscriber.subscribe().unwrap(),
    );

    // FIXME: Unwrapping the init is not nice, maybe return a Result for new()?
    // runner.init().await.unwrap();

    let mut control = Control::new(state_ch, AtHandle(&state.at_handle));
    // control.init().await.unwrap();

    (net_device, control, runner)
}
