use atat::{ResponseSlot, UrcChannel};

use crate::command::Urc;

use super::{runner::URC_SUBSCRIBERS, state};

#[cfg(feature = "cmux")]
use super::runner::{CMUX_CHANNELS, CMUX_CHANNEL_SIZE};

pub struct Resources<
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub(crate) ch: state::State,

    pub(crate) res_slot: ResponseSlot<INGRESS_BUF_SIZE>,
    pub(crate) urc_channel: UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    pub(crate) cmd_buf: [u8; CMD_BUF_SIZE],
    pub(crate) ingress_buf: [u8; INGRESS_BUF_SIZE],

    #[cfg(feature = "cmux")]
    pub(crate) mux: embassy_at_cmux::Mux<CMUX_CHANNELS, CMUX_CHANNEL_SIZE>,
}

impl<const CMD_BUF_SIZE: usize, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize> Default
    for Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const CMD_BUF_SIZE: usize, const INGRESS_BUF_SIZE: usize, const URC_CAPACITY: usize>
    Resources<CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub fn new() -> Self {
        Self {
            ch: state::State::new(),

            res_slot: ResponseSlot::new(),
            urc_channel: UrcChannel::new(),
            cmd_buf: [0; CMD_BUF_SIZE],
            ingress_buf: [0; INGRESS_BUF_SIZE],

            #[cfg(feature = "cmux")]
            mux: embassy_at_cmux::Mux::new(),
        }
    }
}
