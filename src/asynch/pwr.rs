use embassy_time::{Duration, Timer};
use embedded_hal::digital::{InputPin as _, OutputPin as _};

use crate::{
    asynch::state::OperationState,
    config::CellularConfig,
    error::Error,
    modules::{Generic, ModuleParams as _},
};

use super::state;

const GENERIC_PWR_ON_TIMES: [u16; 2] = [300, 2000];

pub(crate) struct PwrCtrl<'a, 'b, C> {
    config: &'b mut C,
    ch: &'b state::Runner<'a>,
}

impl<'a, 'b, C> PwrCtrl<'a, 'b, C>
where
    C: CellularConfig<'a>,
{
    pub(crate) fn new(ch: &'b state::Runner<'a>, config: &'b mut C) -> Self {
        Self { ch, config }
    }

    pub(crate) fn has_power(&mut self) -> Result<bool, Error> {
        if let Some(pin) = self.config.vint_pin() {
            if pin.is_high().map_err(|_| Error::IoPin)? {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            info!("No VInt pin configured");
            Ok(true)
        }
    }

    /// Reset the module by driving it's `RESET_N` pin low for
    /// `Module::reset_hold()` duration
    ///
    /// **NOTE** This function will reset NVM settings!
    pub(crate) async fn reset(&mut self) -> Result<(), Error> {
        warn!("Hard resetting Ublox Cellular Module");
        if let Some(pin) = self.config.reset_pin() {
            pin.set_low().ok();
            Timer::after(
                self.ch
                    .module()
                    .map(|m| m.reset_hold())
                    .unwrap_or(Generic.reset_hold()),
            )
            .await;
            pin.set_high().ok();
            Timer::after(
                self.ch
                    .module()
                    .map(|m| m.boot_wait())
                    .unwrap_or(Generic.boot_wait()),
            )
            .await;
        } else {
            warn!("No reset pin configured");
        }
        Ok(())
    }

    pub(crate) async fn power_up(&mut self) -> Result<(), Error> {
        if !self.has_power()? {
            debug!("Attempting to power up device");

            for generic_time in GENERIC_PWR_ON_TIMES {
                let pull_time = self
                    .ch
                    .module()
                    .map(|m| m.power_on_pull_time())
                    .unwrap_or(Generic.power_on_pull_time())
                    .unwrap_or(Duration::from_millis(generic_time as _));
                if let Some(pin) = self.config.power_pin() {
                    pin.set_low().map_err(|_| Error::IoPin)?;
                    Timer::after(pull_time).await;
                    pin.set_high().map_err(|_| Error::IoPin)?;

                    Timer::after(
                        self.ch
                            .module()
                            .map(|m| m.boot_wait())
                            .unwrap_or(Generic.boot_wait()),
                    )
                    .await;

                    if !self.has_power()? {
                        if self.ch.module().is_some() {
                            return Err(Error::PoweredDown);
                        }
                        continue;
                    }

                    debug!("Powered up");
                    return Ok(());
                } else {
                    warn!("No power pin configured");
                    return Ok(());
                }
            }
            Err(Error::PoweredDown)
        } else {
            Ok(())
        }
    }

    pub(crate) async fn power_down(&mut self) -> Result<(), Error> {
        if self.has_power()? {
            if let Some(pin) = self.config.power_pin() {
                pin.set_low().map_err(|_| Error::IoPin)?;
                Timer::after(
                    self.ch
                        .module()
                        .map(|m| m.power_off_pull_time())
                        .unwrap_or(Generic.power_off_pull_time()),
                )
                .await;
                pin.set_high().map_err(|_| Error::IoPin)?;
                self.ch.set_operation_state(OperationState::PowerDown);
                debug!("Powered down");

                Timer::after_secs(1).await;
            } else {
                warn!("No power pin configured");
            }
        } else {
            self.ch.set_operation_state(OperationState::PowerDown);
        }
        Ok(())
    }
}
