#![allow(dead_code)]

use core::cell::RefCell;
use core::mem::MaybeUninit;
use core::task::Context;

use atat::asynch::AtatClient;
use atat::UrcSubscription;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::pubsub::PubSubChannel;
use embassy_sync::waitqueue::WakerRegistration;

/// The link state of a network device.
#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LinkState {
    /// The link is down.
    Down,
    /// The link is up.
    Up,
}

/// If the celular modem is up and responding to AT.
#[derive(PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PowerState {
    PowerDown,
    PowerUp,
    Alive,
    Initialized,
    Connected,
    DataEstablished,
}

use crate::command::Urc;
use crate::error::Error;

use super::AtHandle;

pub struct State<const MAX_STATE_LISTENERS: usize> {
    inner: MaybeUninit<StateInner<MAX_STATE_LISTENERS>>,
}

impl<const MAX_STATE_LISTENERS: usize> State<MAX_STATE_LISTENERS> {
    pub const fn new() -> Self {
        Self {
            inner: MaybeUninit::uninit(),
        }
    }
}

struct StateInner<const MAX_STATE_LISTENERS: usize> {
    shared: Mutex<NoopRawMutex, RefCell<Shared>>,
    desired_state_pub_sub: PubSubChannel<NoopRawMutex, PowerState, 1, MAX_STATE_LISTENERS, 1>,
}

/// State of the LinkState
pub struct Shared {
    link_state: LinkState,
    power_state: PowerState,
    desired_state: PowerState,
    waker: WakerRegistration,
}

pub struct Runner<'d, const MAX_STATE_LISTENERS: usize> {
    shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
    desired_state_pub_sub: &'d PubSubChannel<NoopRawMutex, PowerState, 1, MAX_STATE_LISTENERS, 1>,
}

#[derive(Clone, Copy)]
pub struct StateRunner<'d, const MAX_STATE_LISTENERS: usize> {
    shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
    desired_state_pub_sub: &'d PubSubChannel<NoopRawMutex, PowerState, 1, MAX_STATE_LISTENERS, 1>,
}

impl<'d, const MAX_STATE_LISTENERS: usize> Runner<'d, MAX_STATE_LISTENERS> {
    pub fn state_runner(&self) -> StateRunner<'d, MAX_STATE_LISTENERS> {
        StateRunner {
            shared: self.shared,
            desired_state_pub_sub: self.desired_state_pub_sub,
        }
    }

    pub fn set_link_state(&mut self, state: LinkState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state = state;
            s.waker.wake();
        });
    }

    pub fn set_power_state(&mut self, state: PowerState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.power_state = state;
            s.waker.wake();
        });
    }

    pub fn set_desired_state(&mut self, ps: PowerState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state = ps;
            s.waker.wake();
        });
        self.desired_state_pub_sub
            .immediate_publisher()
            .publish_immediate(ps);
    }
}

impl<'d, const MAX_STATE_LISTENERS: usize> StateRunner<'d, MAX_STATE_LISTENERS> {
    pub fn set_link_state(&self, state: LinkState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state = state;
            s.waker.wake();
        });
    }

    pub fn link_state_poll_fn(&mut self, cx: &mut Context) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.waker.register(cx.waker());
            s.link_state
        })
    }

    pub fn set_power_state(&self, state: PowerState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.power_state = state;
            s.waker.wake();
        });
    }

    pub fn power_state_poll_fn(&mut self, cx: &mut Context) -> PowerState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.waker.register(cx.waker());
            s.power_state
        })
    }

    pub fn link_state(&mut self) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state
        })
    }

    pub fn power_state(&mut self) -> PowerState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.power_state
        })
    }

    pub fn desired_state(&mut self) -> PowerState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state
        })
    }

    pub async fn set_desired_state(&mut self, ps: PowerState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state = ps;
            s.waker.wake();
        });
        self.desired_state_pub_sub
            .immediate_publisher()
            .publish_immediate(ps);
    }

    pub async fn wait_for_desired_state(&mut self, ps: PowerState) -> Result<PowerState, Error> {
        if self.desired_state() == ps {
            info!("Desired state already set to {:?}, returning", ps);
            return Ok(ps);
        }
        let mut sub = self
            .desired_state_pub_sub
            .subscriber()
            .map_err(|x| Error::SubscriberOverflow(x))?;
        loop {
            let ps_now = sub.next_message_pure().await;
            if ps_now == ps {
                return Ok(ps_now);
            }
        }
    }

    pub async fn wait_for_desired_state_change(&mut self) -> Result<PowerState, Error> {
        let mut sub = self
            .desired_state_pub_sub
            .subscriber()
            .map_err(|x| Error::SubscriberOverflow(x))?;
        Ok(sub.next_message_pure().await)
    }
}

pub fn new<'d, AT: AtatClient, const URC_CAPACITY: usize, const MAX_STATE_LISTENERS: usize>(
    state: &'d mut State<MAX_STATE_LISTENERS>,
    at: AtHandle<'d, AT>,
    urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
) -> (
    Runner<'d, MAX_STATE_LISTENERS>,
    Device<'d, AT, URC_CAPACITY, MAX_STATE_LISTENERS>,
) {
    // safety: this is a self-referential struct, however:
    // - it can't move while the `'d` borrow is active.
    // - when the borrow ends, the dangling references inside the MaybeUninit will never be used again.
    let state_uninit: *mut MaybeUninit<StateInner<MAX_STATE_LISTENERS>> =
        (&mut state.inner as *mut MaybeUninit<StateInner<MAX_STATE_LISTENERS>>).cast();

    let state = unsafe { &mut *state_uninit }.write(StateInner {
        shared: Mutex::new(RefCell::new(Shared {
            link_state: LinkState::Down,
            power_state: PowerState::PowerDown,
            desired_state: PowerState::PowerDown,
            waker: WakerRegistration::new(),
        })),
        desired_state_pub_sub:
            PubSubChannel::<NoopRawMutex, PowerState, 1, MAX_STATE_LISTENERS, 1>::new(),
    });

    (
        Runner {
            shared: &state.shared,
            desired_state_pub_sub: &state.desired_state_pub_sub,
        },
        Device {
            shared: &state.shared,
            urc_subscription,
            at,
            desired_state_pub_sub: &state.desired_state_pub_sub,
        },
    )
}

pub struct Device<'d, AT: AtatClient, const URC_CAPACITY: usize, const MAX_STATE_LISTENERS: usize> {
    pub(crate) shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
    pub(crate) desired_state_pub_sub:
        &'d PubSubChannel<NoopRawMutex, PowerState, 1, MAX_STATE_LISTENERS, 1>,
    pub(crate) at: AtHandle<'d, AT>,
    pub(crate) urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
}

impl<'d, AT: AtatClient, const URC_CAPACITY: usize, const MAX_STATE_LISTENERS: usize>
    Device<'d, AT, URC_CAPACITY, MAX_STATE_LISTENERS>
{
    pub fn link_state_poll_fn(&mut self, cx: &mut Context) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.waker.register(cx.waker());
            s.link_state
        })
    }

    pub fn power_state_poll_fn(&mut self, cx: &mut Context) -> PowerState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.waker.register(cx.waker());
            s.power_state
        })
    }

    pub fn link_state(&mut self) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state
        })
    }

    pub fn power_state(&mut self) -> PowerState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.power_state
        })
    }

    pub fn desired_state(&mut self) -> PowerState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state
        })
    }

    pub fn set_desired_state(&mut self, ps: PowerState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state = ps;
            s.waker.wake();
        });
        self.desired_state_pub_sub
            .immediate_publisher()
            .publish_immediate(ps);
    }

    pub async fn wait_for_desired_state(&mut self, ps: PowerState) -> Result<PowerState, Error> {
        if self.desired_state() == ps {
            return Ok(ps);
        }
        let mut sub = self
            .desired_state_pub_sub
            .subscriber()
            .map_err(|x| Error::SubscriberOverflow(x))?;
        loop {
            let ps_now = sub.next_message_pure().await;
            if ps_now == ps {
                return Ok(ps_now);
            }
        }
    }
}
