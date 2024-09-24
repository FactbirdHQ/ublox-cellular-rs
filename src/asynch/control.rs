use core::cell::Cell;

use atat::{asynch::AtatClient, response_slot::ResponseSlotGuard};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender};
use embassy_time::{with_timeout, Duration, Timer};

use crate::{
    command::{
        general::{types::FirmwareVersion, GetFirmwareVersion},
        gpio::{types::GpioMode, SetGpioConfiguration},
        network_service::{
            responses::{OperatorSelection, SignalQuality},
            GetOperatorSelection, GetSignalQuality,
        },
    },
    error::Error,
};

use super::{
    runner::MAX_CMD_LEN,
    state::{self, LinkState, OperationState},
};

pub(crate) struct ProxyClient<'a, const INGRESS_BUF_SIZE: usize> {
    pub(crate) req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
    pub(crate) res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    cooldown_timer: Cell<Option<Timer>>,
}

impl<'a, const INGRESS_BUF_SIZE: usize> ProxyClient<'a, INGRESS_BUF_SIZE> {
    pub fn new(
        req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    ) -> Self {
        Self {
            req_sender,
            res_slot,
            cooldown_timer: Cell::new(None),
        }
    }

    async fn wait_response(
        &self,
        timeout: Duration,
    ) -> Result<ResponseSlotGuard<'_, INGRESS_BUF_SIZE>, atat::Error> {
        with_timeout(timeout, self.res_slot.get())
            .await
            .map_err(|_| atat::Error::Timeout)
    }
}

impl<'a, const INGRESS_BUF_SIZE: usize> atat::asynch::AtatClient
    for &ProxyClient<'a, INGRESS_BUF_SIZE>
{
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
        let mut buf = [0u8; MAX_CMD_LEN];
        let len = cmd.write(&mut buf);

        if len < 50 {
            trace!(
                "Sending command: {:?}",
                atat::helpers::LossyStr(&buf[..len])
            );
        } else {
            trace!("Sending command with long payload ({} bytes)", len);
        }

        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.await
        }

        // TODO: Guard against race condition!
        with_timeout(
            Duration::from_secs(1),
            self.req_sender
                .send(heapless::Vec::try_from(&buf[..len]).unwrap()),
        )
        .await
        .map_err(|_| atat::Error::Timeout)?;

        self.cooldown_timer.set(Some(Timer::after_millis(20)));

        if !Cmd::EXPECTS_RESPONSE_CODE {
            cmd.parse(Ok(&[]))
        } else {
            let response = self
                .wait_response(Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))
                .await?;
            let response: &atat::Response<INGRESS_BUF_SIZE> = &response.borrow();
            cmd.parse(response.into())
        }
    }
}

pub struct Control<'a, const INGRESS_BUF_SIZE: usize> {
    state_ch: state::Runner<'a>,
    at_client: ProxyClient<'a, INGRESS_BUF_SIZE>,
}

impl<'a, const INGRESS_BUF_SIZE: usize> Control<'a, INGRESS_BUF_SIZE> {
    pub(crate) fn new(
        state_ch: state::Runner<'a>,
        req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    ) -> Self {
        Self {
            state_ch,
            at_client: ProxyClient::new(req_sender, res_slot),
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
        Ok(self.send(&GetSignalQuality).await?)
    }

    pub async fn get_operator(&self) -> Result<OperatorSelection, Error> {
        Ok(self.send(&GetOperatorSelection).await?)
    }

    pub async fn get_version(&self) -> Result<FirmwareVersion, Error> {
        let res = self.send(&GetFirmwareVersion).await?;
        Ok(res.version)
    }

    pub async fn set_gpio_configuration(
        &self,
        gpio_id: u8,
        gpio_mode: GpioMode,
    ) -> Result<(), Error> {
        self.send(&SetGpioConfiguration { gpio_id, gpio_mode })
            .await?;
        Ok(())
    }

    /// Send an AT command to the modem This is useful if you have special
    /// configuration but might break the drivers functionality if your settings
    /// interfere with the drivers settings
    pub async fn send<Cmd: atat::AtatCmd>(&self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        Ok((&self.at_client).send_retry::<Cmd>(cmd).await?)
    }
}
