use core::future::poll_fn;
use core::task::Poll;

use atat::asynch::AtatClient;
use embassy_time::{with_timeout, Duration};

use crate::error::Error;

use super::state::{LinkState, PowerState};
use super::{state, AtHandle};

pub struct Control<'a, AT: AtatClient, const MAX_STATE_LISTENERS: usize> {
    state_ch: state::StateRunner<'a, MAX_STATE_LISTENERS>,
    at: AtHandle<'a, AT>,
}

impl<'a, AT: AtatClient, const MAX_STATE_LISTENERS: usize> Control<'a, AT, MAX_STATE_LISTENERS> {
    pub(crate) fn new(
        state_ch: state::StateRunner<'a, MAX_STATE_LISTENERS>,
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

    pub fn power_state(&mut self) -> PowerState {
        self.state_ch.power_state()
    }

    pub fn desired_state(&mut self) -> PowerState {
        self.state_ch.desired_state()
    }

    pub async fn set_desired_state(&mut self, ps: PowerState) {
        self.state_ch.set_desired_state(ps).await;
    }
}
