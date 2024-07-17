#![allow(dead_code)]

use core::cell::RefCell;
use core::future::poll_fn;
use core::task::{Context, Poll};

use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
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
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OperationState {
    PowerDown = 0,
    Initialized = 1,
    Connected = 2,
    DataEstablished = 3,
}

use crate::modules::Module;
use crate::registration::{ProfileState, RegistrationState};

pub struct State {
    shared: Mutex<NoopRawMutex, RefCell<Shared>>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub const fn new() -> Self {
        Self {
            shared: Mutex::new(RefCell::new(Shared {
                link_state: LinkState::Down,
                operation_state: OperationState::PowerDown,
                module: None,
                desired_state: OperationState::Initialized,
                registration_state: RegistrationState::new(),
                state_waker: WakerRegistration::new(),
                registration_waker: WakerRegistration::new(),
            })),
        }
    }
}

/// State of the LinkState
pub struct Shared {
    link_state: LinkState,
    operation_state: OperationState,
    desired_state: OperationState,
    module: Option<Module>,
    registration_state: RegistrationState,
    state_waker: WakerRegistration,
    registration_waker: WakerRegistration,
}

#[derive(Clone)]
pub struct Runner<'d> {
    pub(crate) shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
}

impl<'d> Runner<'d> {
    pub fn new(state: &'d mut State) -> Self {
        Self {
            shared: &state.shared,
        }
    }

    pub(crate) fn module(&self) -> Option<Module> {
        self.shared.lock(|s| s.borrow().module)
    }

    pub(crate) fn set_module(&self, module: Module) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.module.replace(module);
        });
    }

    pub fn update_registration_with(&self, f: impl FnOnce(&mut RegistrationState)) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            f(&mut s.registration_state);
            info!(
                "Registration status changed! Registered: {:?}",
                s.registration_state.is_registered()
            );
            s.registration_waker.wake();
        })
    }

    pub fn is_registered(&self, cx: Option<&mut Context>) -> bool {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.registration_waker.register(cx.waker());
            }
            s.registration_state.is_registered()
        })
    }

    #[cfg(not(feature = "use-upsd-context-activation"))]
    pub fn set_profile_state(&self, state: ProfileState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.registration_state.profile_state = state;
        })
    }

    #[cfg(not(feature = "use-upsd-context-activation"))]
    pub fn get_profile_state(&self) -> ProfileState {
        self.shared
            .lock(|s| s.borrow().registration_state.profile_state)
    }

    pub fn set_link_state(&self, state: LinkState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state = state;
            s.state_waker.wake();
        });
    }

    pub fn link_state(&self, cx: Option<&mut Context>) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.state_waker.register(cx.waker());
            }
            s.link_state
        })
    }

    pub fn set_operation_state(&self, state: OperationState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.operation_state = state;
            s.state_waker.wake();
        });
    }

    pub fn operation_state(&self, cx: Option<&mut Context>) -> OperationState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.state_waker.register(cx.waker());
            }
            s.operation_state
        })
    }

    pub fn set_desired_state(&self, ps: OperationState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state = ps;
            s.state_waker.wake();
        });
    }

    pub fn desired_state(&self, cx: Option<&mut Context>) -> OperationState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            if let Some(cx) = cx {
                s.state_waker.register(cx.waker());
            }
            s.desired_state
        })
    }

    pub async fn wait_for_desired_state(&self, ps: OperationState) {
        if self.desired_state(None) == ps {
            return;
        }

        poll_fn(|cx| {
            if self.desired_state(Some(cx)) == ps {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await
    }

    pub async fn wait_for_operation_state(&self, ps: OperationState) {
        if self.operation_state(None) == ps {
            return;
        }

        poll_fn(|cx| {
            if self.operation_state(Some(cx)) == ps {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await
    }

    pub async fn wait_for_desired_state_change(&self) -> OperationState {
        let old_desired = self.desired_state(None);

        poll_fn(|cx| {
            let current_desired = self.desired_state(Some(cx));
            if current_desired != old_desired {
                return Poll::Ready(current_desired);
            }
            Poll::Pending
        })
        .await
    }

    pub async fn wait_registration_change(&self) -> bool {
        let old_state = self.is_registered(None);

        poll_fn(|cx| {
            let current_state = self.is_registered(Some(cx));
            if current_state != old_state {
                return Poll::Ready(current_state);
            }
            Poll::Pending
        })
        .await
    }
}

#[cfg(feature = "internal-network-stack")]
pub struct Device<'d, const URC_CAPACITY: usize> {
    pub(crate) shared: &'d Mutex<NoopRawMutex, RefCell<Shared>>,
    // pub(crate) at: AtHandle<'d, AT>,
    pub(crate) urc_subscription: UrcSubscription<'d, Urc, URC_CAPACITY, 2>,
}

#[cfg(feature = "internal-network-stack")]
impl<'d, const URC_CAPACITY: usize> Device<'d, URC_CAPACITY> {
    pub fn link_state(&self, cx: &mut Context) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.state_waker.register(cx.waker());
            s.link_state
        })
    }

    pub fn operation_state(&self, cx: &mut Context) -> OperationState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.state_waker.register(cx.waker());
            s.operation_state
        })
    }

    pub fn link_state(&self) -> LinkState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.link_state
        })
    }

    pub fn operation_state(&self) -> OperationState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.operation_state
        })
    }

    pub fn desired_state(&self, cx: &mut Context) -> OperationState {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.state_waker.register(cx.waker());
            s.desired_state
        })
    }

    pub fn set_desired_state(&self, ps: OperationState) {
        self.shared.lock(|s| {
            let s = &mut *s.borrow_mut();
            s.desired_state = ps;
            s.state_waker.wake();
        });
    }

    pub async fn wait_for_desired_state(&self, ps: OperationState) {
        poll_fn(|cx| {
            if self.desired_state(cx) == ps {
                return Poll::Ready(());
            }
            Poll::Pending
        })
        .await
    }

    pub async fn wait_for_desired_state_change(&self) -> OperationState {
        let current_desired = self.shared.lock(|s| s.borrow().desired_state);

        poll_fn(|cx| {
            if self.desired_state(cx) != current_desired {
                return Poll::Ready(ps);
            }
            Poll::Pending
        })
        .await
    }
}
