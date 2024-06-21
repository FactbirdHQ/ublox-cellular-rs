use super::state::{self, LinkState, OperationState};

pub struct Control<'a> {
    state_ch: state::Runner<'a>,
}

impl<'a> Control<'a> {
    pub(crate) fn new(state_ch: state::Runner<'a>) -> Self {
        Self { state_ch }
    }

    pub fn link_state(&mut self) -> LinkState {
        self.state_ch.link_state(None)
    }

    pub fn operation_state(&mut self) -> OperationState {
        self.state_ch.operation_state(None)
    }

    pub fn desired_state(&mut self) -> OperationState {
        self.state_ch.desired_state(None)
    }

    pub fn set_desired_state(&mut self, ps: OperationState) {
        self.state_ch.set_desired_state(ps);
    }

    pub async fn wait_for_desired_state(&mut self, ps: OperationState) {
        self.state_ch.wait_for_desired_state(ps).await
    }

    pub async fn wait_for_operation_state(&mut self, ps: OperationState) {
        self.state_ch.wait_for_operation_state(ps).await
    }

    // pub async fn get_signal_quality(
    //     &mut self,
    // ) -> Result<crate::command::network_service::responses::SignalQuality, Error> {
    //     self.at
    //         .send(&crate::command::network_service::GetSignalQuality)
    //         .await
    //         .map_err(|e| Error::Atat(e))
    // }

    // pub async fn get_operator(
    //     &mut self,
    // ) -> Result<crate::command::network_service::responses::OperatorSelection, Error> {
    //     self.at
    //         .send(&crate::command::network_service::GetOperatorSelection)
    //         .await
    //         .map_err(|e| Error::Atat(e))
    // }

    // /// Send an AT command to the modem
    // /// This is usefull if you have special configuration but might break the drivers functionality if your settings interfere with the drivers settings
    // pub async fn send<Cmd: atat::AtatCmd>(
    //     &mut self,
    //     cmd: &Cmd,
    // ) -> Result<Cmd::Response, atat::Error> {
    //     self.at.send::<Cmd>(cmd).await
    // }
}
