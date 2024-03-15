use core::mem::MaybeUninit;

use atat::{asynch::Client, ResponseSlot, UrcChannel};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embedded_io_async::Write;

use crate::command::Urc;

use super::{runner::URC_SUBSCRIBERS, state};

pub struct UbxResources<
    W: Write,
    const CMD_BUF_SIZE: usize,
    const INGRESS_BUF_SIZE: usize,
    const URC_CAPACITY: usize,
> {
    pub(crate) ch: state::State,

    pub(crate) res_slot: ResponseSlot<INGRESS_BUF_SIZE>,
    pub(crate) urc_channel: UrcChannel<Urc, URC_CAPACITY, URC_SUBSCRIBERS>,
    pub(crate) cmd_buf: [u8; CMD_BUF_SIZE],
    pub(crate) ingress_buf: [u8; INGRESS_BUF_SIZE],

    pub(crate) at_client: MaybeUninit<Mutex<NoopRawMutex, Client<'static, W, INGRESS_BUF_SIZE>>>,

    #[cfg(feature = "ppp")]
    pub(crate) ppp_state: embassy_net_ppp::State<2, 2>,

    #[cfg(feature = "ppp")]
    pub(crate) mux:
        embassy_at_cmux::Mux<{ super::ppp::CMUX_CHANNELS }, { super::ppp::CMUX_CHANNEL_SIZE }>,
}

impl<
        W: Write,
        const CMD_BUF_SIZE: usize,
        const INGRESS_BUF_SIZE: usize,
        const URC_CAPACITY: usize,
    > UbxResources<W, CMD_BUF_SIZE, INGRESS_BUF_SIZE, URC_CAPACITY>
{
    pub fn new() -> Self {
        Self {
            ch: state::State::new(),

            res_slot: ResponseSlot::new(),
            urc_channel: UrcChannel::new(),
            cmd_buf: [0; CMD_BUF_SIZE],
            ingress_buf: [0; INGRESS_BUF_SIZE],

            at_client: MaybeUninit::uninit(),

            #[cfg(feature = "ppp")]
            ppp_state: embassy_net_ppp::State::new(),

            #[cfg(feature = "ppp")]
            mux: embassy_at_cmux::Mux::new(),
        }
    }
}
