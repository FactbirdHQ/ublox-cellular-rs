use atat::asynch::{AtatClient, SimpleClient};
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::Mutex};

use crate::{
    command::{
        general::{types::FirmwareVersion, GetFirmwareVersion},
        gpio::{types::GpioMode, SetGpioConfiguration},
        network_service::{
            responses::{OperatorSelection, SignalQuality},
            GetOperatorSelection, GetSignalQuality,
        },
        Urc,
    },
    error::Error,
};

use super::{
    runner::CMUX_CHANNEL_SIZE,
    state::{self, LinkState, OperationState},
};

pub struct Control<'a, M: RawMutex> {
    state_ch: state::Runner<'a>,
    at_client: Mutex<
        M,
        SimpleClient<'a, embassy_at_cmux::Channel<'a, CMUX_CHANNEL_SIZE>, atat::AtDigester<Urc>>,
    >,
}

impl<'a, M: RawMutex> Control<'a, M> {
    pub(crate) fn new(
        state_ch: state::Runner<'a>,
        at_client: SimpleClient<
            'a,
            embassy_at_cmux::Channel<'a, CMUX_CHANNEL_SIZE>,
            atat::AtDigester<Urc>,
        >,
    ) -> Self {
        Self {
            state_ch,
            at_client: Mutex::new(at_client),
        }
    }

    pub fn link_state(&self) -> LinkState {
        self.state_ch.link_state(None)
    }

    pub fn operation_state(&self) -> OperationState {
        self.state_ch.operation_state(None)
    }

    pub fn desired_state(&self) -> OperationState {
        self.state_ch.desired_state(None)
    }

    pub fn set_desired_state(&self, ps: OperationState) {
        self.state_ch.set_desired_state(ps);
    }

    pub async fn wait_for_desired_state(&self, ps: OperationState) {
        self.state_ch.wait_for_desired_state(ps).await
    }

    pub async fn wait_for_operation_state(&self, ps: OperationState) {
        self.state_ch.wait_for_operation_state(ps).await
    }

    pub async fn get_signal_quality(&self) -> Result<SignalQuality, Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        Ok(self
            .at_client
            .lock()
            .await
            .send_retry(&GetSignalQuality)
            .await?)
    }

    pub async fn get_operator(&self) -> Result<OperatorSelection, Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        Ok(self
            .at_client
            .lock()
            .await
            .send_retry(&GetOperatorSelection)
            .await?)
    }

    pub async fn get_version(&self) -> Result<FirmwareVersion, Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        let res = self
            .at_client
            .lock()
            .await
            .send_retry(&GetFirmwareVersion)
            .await?;
        Ok(res.version)
    }

    pub async fn set_gpio_configuration(
        &self,
        gpio_id: u8,
        gpio_mode: GpioMode,
    ) -> Result<(), Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        self.at_client
            .lock()
            .await
            .send_retry(&SetGpioConfiguration { gpio_id, gpio_mode })
            .await?;
        Ok(())
    }

    /// Send an AT command to the modem This is usefull if you have special
    /// configuration but might break the drivers functionality if your settings
    /// interfere with the drivers settings
    pub async fn send<Cmd: atat::AtatCmd>(&self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        Ok(self.at_client.lock().await.send_retry::<Cmd>(cmd).await?)
    }
}
