pub mod control;
mod resources;
pub mod runner;
pub mod state;

#[cfg(feature = "internal-network-stack")]
mod internal_stack;
#[cfg(feature = "internal-network-stack")]
pub use internal_stack::{new_internal, InternalRunner, Resources};

#[cfg(feature = "ppp")]
mod ppp;
#[cfg(feature = "ppp")]
pub use ppp::{new_ppp, PPPRunner, Resources};

use atat::asynch::AtatClient;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};

pub struct AtHandle<'d, AT: AtatClient>(&'d Mutex<NoopRawMutex, AT>);

impl<'d, AT: AtatClient> AtHandle<'d, AT> {
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
        self.0.lock().await.send_retry::<Cmd>(cmd).await
    }
}
