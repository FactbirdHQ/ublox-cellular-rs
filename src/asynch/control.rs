use core::cell::Cell;

use atat::{asynch::AtatClient, response_slot::ResponseSlotGuard};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender, mutex::Mutex};
use embassy_time::{with_timeout, Duration, Timer};

use crate::{
    command::{
        general::{types::FirmwareVersion, GetCCID, GetFirmwareVersion},
        gpio::{types::GpioMode, ReadGpioPin, SetGpioConfiguration},
        network_service::{
            responses::{OperatorSelection, SignalQuality},
            types::RatAct,
            GetOperatorSelection, GetSignalQuality,
        },
        psn::GetPDPContextDefinition,
    },
    config::Apn,
    error::Error,
};

use super::{
    runner::MAX_CMD_LEN,
    state::{self, LinkState, OperationState},
};

pub(crate) struct ProxyClient<'a, const INGRESS_BUF_SIZE: usize> {
    pub(crate) req_sender:
        Mutex<NoopRawMutex, Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>>,
    pub(crate) res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    cooldown_timer: Cell<Option<Timer>>,
}

impl<'a, const INGRESS_BUF_SIZE: usize> ProxyClient<'a, INGRESS_BUF_SIZE> {
    pub fn new(
        req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    ) -> Self {
        Self {
            req_sender: Mutex::new(req_sender),
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
            info!("🔧 AT Command: {:?}", atat::helpers::LossyStr(&buf[..len]));
        } else {
            info!("🔧 AT Command: Long payload ({} bytes)", len);
            debug!(
                "AT Command payload: {:?}",
                atat::helpers::LossyStr(&buf[..len.min(200)])
            );
        }

        if let Some(cooldown) = self.cooldown_timer.take() {
            cooldown.await
        }

        let sender = self.req_sender.lock().await;

        with_timeout(
            Duration::from_secs(1),
            sender.send(heapless::Vec::try_from(&buf[..len]).unwrap()),
        )
        .await
        .map_err(|_| atat::Error::Timeout)?;

        self.cooldown_timer.set(Some(Timer::after_millis(20)));

        if !Cmd::EXPECTS_RESPONSE_CODE {
            debug!("AT Command expects no response, parsing empty response");
            drop(sender);
            cmd.parse(Ok(&[]))
        } else {
            debug!(
                "AT Command expects response, waiting up to {}ms",
                Cmd::MAX_TIMEOUT_MS
            );
            let response = self
                .wait_response(Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))
                .await?;

            // Release sender lock after receiving response
            drop(sender);

            let response: &atat::Response<INGRESS_BUF_SIZE> = &response.borrow();
            let response_result: Result<&[u8], _> = response.into();
            if let Ok(response_bytes) = &response_result {
                if response_bytes.len() < 200 {
                    debug!(
                        "📡 AT Response: {:?}",
                        atat::helpers::LossyStr(response_bytes)
                    );
                } else {
                    debug!(
                        "📡 AT Response: Long response ({} bytes): {:?}",
                        response_bytes.len(),
                        atat::helpers::LossyStr(&response_bytes[..200.min(response_bytes.len())])
                    );
                }
            }
            cmd.parse(response_result)
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

    pub fn is_connected(&self) -> bool {
        self.link_state() == LinkState::Up
    }

    pub async fn is_denied(&self) -> bool {
        self.state_ch.is_denied(None)
    }

    pub fn desired_state(&self) -> OperationState {
        self.state_ch.desired_state(None)
    }

    pub fn set_desired_state(&self, ps: OperationState) {
        self.state_ch.set_desired_state(ps);
    }

    pub fn set_apn_config(&self, apn: Apn) {
        self.state_ch.set_apn_config(apn);
    }

    pub async fn wait_for_link_state(&self, link_state: LinkState) {
        self.state_ch.wait_for_link_state(link_state).await;
    }

    pub async fn wait_for_desired_state(&self, ps: OperationState) {
        self.state_ch.wait_for_desired_state(ps).await
    }

    pub async fn wait_for_operation_state(&self, ps: OperationState) {
        self.state_ch.wait_for_operation_state(ps).await
    }

    /// Get the current Radio Access Technology (2G/3G/4G etc.)
    pub fn current_rat(&self) -> Option<RatAct> {
        self.state_ch.current_rat(None)
    }

    /// Wait for the Radio Access Technology to change (e.g., 3G -> 4G)
    /// Returns the new RAT value when it changes
    pub async fn wait_rat_change(&self) -> Option<RatAct> {
        self.state_ch.wait_rat_change().await
    }

    /// Wait for either DataEstablished state or powered down indicating something went bad.
    /// Returns Ok(()) if DataEstablished is reached, or Error if registration is denied.
    pub async fn wait_for_data_established_or_powered_down(&self) -> Result<(), Error> {
        use core::task::Poll;
        use embassy_futures::select::{select, Either};

        let state_runner = self.state_ch.clone();

        let wait_for_data_established = core::future::poll_fn(|cx| {
            if state_runner.operation_state(Some(cx)) == OperationState::DataEstablished {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        });

        let wait_for_powered_down = core::future::poll_fn(|cx| {
            if state_runner.operation_state(Some(cx)) == OperationState::PowerDown {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        });

        match select(wait_for_data_established, wait_for_powered_down).await {
            Either::First(_) => {
                info!("✅ Data connection established successfully");
                Ok(())
            }
            Either::Second(_) => {
                error!("❌ Module powered down while waiting for data connection");
                Err(Error::Network(
                    crate::command::network_service::types::Error::RegistrationDenied,
                ))
            }
        }
    }

    pub async fn get_signal_quality(&self) -> Result<SignalQuality, Error> {
        self.send(&GetSignalQuality).await
    }

    pub async fn get_operator(&self) -> Result<OperatorSelection, Error> {
        self.send(&GetOperatorSelection).await
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

    pub async fn get_apn_info(&self) -> Result<heapless::String<62>, Error> {
        let pdp_context = self.send(&GetPDPContextDefinition).await?;

        if let Some(config) = pdp_context.first() {
            Ok(config.apn.clone())
        } else {
            Err(Error::_Unknown)
        }
    }

    pub async fn get_ccid(&self) -> Result<u128, Error> {
        let ccid = self.send(&GetCCID).await?;

        Ok(ccid.ccid)
    }
    pub async fn get_gpio_value(&self, gpio_id: u8) -> Result<u8, Error> {
        let value = self.send(&ReadGpioPin { gpio_id }).await?;

        Ok(value.gpio_val)
    }
}
