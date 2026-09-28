use atat::{asynch::AtatClient, response_slot::ResponseSlotGuard};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel::Sender, mutex::Mutex};
use embassy_time::{with_timeout, Duration, Timer};
use embedded_io_async::Write;

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

/// Writer half of [`ProxyClient`].
///
/// Bytes are buffered in `MAX_CMD_LEN` chunks and handed to the runner's AT
/// bridge through the request channel, which writes them to the modem in
/// order. A chunk is sent when the buffer fills up or on `flush`, so payloads
/// larger than `MAX_CMD_LEN` can be streamed through `AtatClient::send_with`.
pub struct ProxyWriter<'a> {
    req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
    buf: heapless::Vec<u8, MAX_CMD_LEN>,
}

impl ProxyWriter<'_> {
    async fn send_chunk(&mut self) -> Result<(), atat::Error> {
        if self.buf.is_empty() {
            return Ok(());
        }

        let chunk = core::mem::take(&mut self.buf);

        // The bridge drains the channel as fast as it can write to the modem;
        // if it is not running, fail rather than blocking the caller forever.
        with_timeout(Duration::from_secs(1), self.req_sender.send(chunk))
            .await
            .map_err(|_| atat::Error::Timeout)
    }
}

impl embedded_io_async::ErrorType for ProxyWriter<'_> {
    type Error = atat::Error;
}

impl Write for ProxyWriter<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.buf.is_full() {
            self.send_chunk().await?;
        }

        let n = buf.len().min(self.buf.capacity() - self.buf.len());
        self.buf
            .extend_from_slice(&buf[..n])
            .map_err(|_| atat::Error::Write)?;
        Ok(n)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.send_chunk().await
    }
}

/// State shared by every [`ProxyClient`] on the same AT link.
///
/// The runner's network device and the user's [`Control`] each own a
/// `ProxyClient`, but they share one request channel and one response slot.
/// Holding this state's mutex for the whole of a request, from the first byte
/// queued until the response is parsed, keeps their commands from
/// interleaving on the wire and from stealing each other's responses.
pub(crate) struct ProxyState<const CMD_BUF_SIZE: usize> {
    /// Modem cooldown after the previous response; awaited before the next
    /// request regardless of which client issues it.
    cooldown_timer: Option<Timer>,
    /// Serialisation buffer for `AtatClient::send`. Sized like the ingress
    /// buffer, as the commands carrying large payloads (file and security data
    /// writes, socket writes) come paired with responses of the same size.
    cmd_buf: [u8; CMD_BUF_SIZE],
}

impl<const CMD_BUF_SIZE: usize> ProxyState<CMD_BUF_SIZE> {
    pub(crate) const fn new() -> Self {
        Self {
            cooldown_timer: None,
            cmd_buf: [0; CMD_BUF_SIZE],
        }
    }
}

/// AT client that forwards commands to the runner's AT bridge over the request
/// channel and reads the parsed result back from the shared response slot.
pub(crate) struct ProxyClient<'a, const INGRESS_BUF_SIZE: usize> {
    writer: ProxyWriter<'a>,
    res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
    state: &'a Mutex<NoopRawMutex, ProxyState<INGRESS_BUF_SIZE>>,
}

impl<'a, const INGRESS_BUF_SIZE: usize> ProxyClient<'a, INGRESS_BUF_SIZE> {
    pub const fn new(
        req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
        state: &'a Mutex<NoopRawMutex, ProxyState<INGRESS_BUF_SIZE>>,
    ) -> Self {
        Self {
            writer: ProxyWriter {
                req_sender,
                buf: heapless::Vec::new(),
            },
            res_slot,
            state,
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

    /// Run one request while the caller holds the shared [`ProxyState`] lock.
    async fn request<Cmd: atat::AtatCmd>(
        &mut self,
        cooldown_timer: &mut Option<Timer>,
        cmd: &Cmd,
        write: impl AsyncFnOnce(&mut ProxyWriter<'a>) -> Result<(), atat::Error>,
    ) -> Result<Cmd::Response, atat::Error> {
        if let Some(cooldown) = cooldown_timer.take() {
            cooldown.await;
        }

        // Discard any bytes left behind by a request whose writer closure
        // failed part-way, so they are not prepended to this request.
        self.writer.buf.clear();

        // Clear any stale response signal left over from prior commands or
        // late URC-like traffic, so wait_response below returns our command's
        // response and not a leaked one.
        self.res_slot.reset();

        write(&mut self.writer).await?;
        self.writer.flush().await?;

        *cooldown_timer = Some(Timer::after_millis(20));

        if !Cmd::EXPECTS_RESPONSE_CODE {
            debug!("AT Command expects no response, parsing empty response");
            return cmd.parse(Ok(&[]));
        }

        debug!(
            "AT Command expects response, waiting up to {}ms",
            Cmd::MAX_TIMEOUT_MS
        );
        let response = self
            .wait_response(Duration::from_millis(Cmd::MAX_TIMEOUT_MS.into()))
            .await?;

        let response: &atat::Response<INGRESS_BUF_SIZE> = &response;
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

impl<'a, const INGRESS_BUF_SIZE: usize> AtatClient for ProxyClient<'a, INGRESS_BUF_SIZE> {
    type Writer = ProxyWriter<'a>;

    fn inner(&mut self) -> &mut Self::Writer {
        &mut self.writer
    }

    async fn send_with<Cmd: atat::AtatCmd>(
        &mut self,
        cmd: &Cmd,
        write: impl AsyncFnOnce(&mut Self::Writer) -> Result<(), atat::Error>,
    ) -> Result<Cmd::Response, atat::Error> {
        // Held until the response is parsed; see `ProxyState`.
        let mut state = self.state.lock().await;
        self.request(&mut state.cooldown_timer, cmd, write).await
    }

    /// Serialises the command into the shared command buffer and streams it
    /// to the modem. Commands larger than `INGRESS_BUF_SIZE` cannot be sent
    /// this way; use `send_with` and write the payload directly instead.
    async fn send<Cmd: atat::AtatCmd>(&mut self, cmd: &Cmd) -> Result<Cmd::Response, atat::Error> {
        // Held until the response is parsed; see `ProxyState`.
        let mut state = self.state.lock().await;
        let ProxyState {
            cooldown_timer,
            cmd_buf,
        } = &mut *state;

        let len = cmd.write(cmd_buf);

        if len < 50 {
            info!(
                "🔧 AT Command: {:?}",
                atat::helpers::LossyStr(&cmd_buf[..len])
            );
        } else {
            info!("🔧 AT Command: Long payload ({} bytes)", len);
            debug!(
                "AT Command payload: {:?}",
                atat::helpers::LossyStr(&cmd_buf[..len.min(200)])
            );
        }

        self.request(cooldown_timer, cmd, async |writer| {
            writer.write_all(&cmd_buf[..len]).await
        })
        .await
    }
}

pub struct Control<'a, const INGRESS_BUF_SIZE: usize> {
    state_ch: state::Runner<'a>,
    /// Behind a mutex only so `send` can take `&self`; the ordering against
    /// the runner's own client is done by the shared [`ProxyState`] lock.
    at_client: Mutex<NoopRawMutex, ProxyClient<'a, INGRESS_BUF_SIZE>>,
}

impl<'a, const INGRESS_BUF_SIZE: usize> Control<'a, INGRESS_BUF_SIZE> {
    pub(crate) const fn new(
        state_ch: state::Runner<'a>,
        req_sender: Sender<'a, NoopRawMutex, heapless::Vec<u8, MAX_CMD_LEN>, 1>,
        res_slot: &'a atat::ResponseSlot<INGRESS_BUF_SIZE>,
        proxy_state: &'a Mutex<NoopRawMutex, ProxyState<INGRESS_BUF_SIZE>>,
    ) -> Self {
        Self {
            state_ch,
            at_client: Mutex::new(ProxyClient::new(req_sender, res_slot, proxy_state)),
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

    /// Make the next power-down skip the graceful AT teardown and hard
    /// power-cycle via GPIO. Call before driving the state to `PowerDown` when
    /// the modem is known unresponsive (e.g. the firmware keepalive), so the
    /// COPS=2/CFUN teardown doesn't burn ~20s of AT timeouts on a dead modem.
    pub fn request_hard_reset(&self) {
        self.state_ch.request_hard_reset();
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
    ///
    /// The serialised command must fit in `INGRESS_BUF_SIZE` bytes. For
    /// larger payloads use [`Control::send_with`].
    pub async fn send<Cmd: atat::AtatCmd>(&self, cmd: &Cmd) -> Result<Cmd::Response, Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        Ok(self.at_client.lock().await.send_retry::<Cmd>(cmd).await?)
    }

    /// Send an AT command whose bytes are produced by `write` instead of by
    /// `AtatCmd::write`. The response is still parsed as `cmd`'s response,
    /// so `cmd` only needs to describe the expected reply and timeout.
    ///
    /// Use this for commands carrying payloads that do not fit the command
    /// buffer, e.g. binary socket writes, file downloads or security data
    /// imports: write the payload straight to the writer, which streams it to
    /// the modem in chunks.
    pub async fn send_with<Cmd: atat::AtatCmd>(
        &self,
        cmd: &Cmd,
        write: impl AsyncFnOnce(&mut ProxyWriter<'a>) -> Result<(), atat::Error>,
    ) -> Result<Cmd::Response, Error> {
        if self.operation_state() == OperationState::PowerDown {
            return Err(Error::Uninitialized);
        }

        Ok(self.at_client.lock().await.send_with(cmd, write).await?)
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
