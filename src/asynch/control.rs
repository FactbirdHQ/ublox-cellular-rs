use core::future::poll_fn;
use core::task::Poll;

use atat::asynch::AtatClient;
use embassy_time::{with_timeout, Duration};

use crate::error::Error;

use super::state::{LinkState, OperationState};
use super::{state, AtHandle};

pub struct Control<'a, AT: AtatClient> {
    state_ch: state::StateRunner<'a>,
    at: AtHandle<'a, AT>,
}

impl<'a, AT: AtatClient> Control<'a, AT> {
    pub(crate) fn new(
        state_ch: state::StateRunner<'a>,
        at: AtHandle<'a, AT>,
    ) -> Self {
        Self { state_ch, at }
    }

    pub(crate) async fn init(&mut self) -> Result<(), Error> {
        debug!("Initalizing ublox control");

        Ok(())
    }

    pub fn link_state(&mut self) -> LinkState {
        self.state_ch.link_state()
    }

    pub fn power_state(&mut self) -> OperationState {
        self.state_ch.power_state()
    }

    pub fn desired_state(&mut self) -> OperationState {
        self.state_ch.desired_state()
    }

    pub async fn set_desired_state(&mut self, ps: OperationState) {
        self.state_ch.set_desired_state(ps).await;
    }
}
