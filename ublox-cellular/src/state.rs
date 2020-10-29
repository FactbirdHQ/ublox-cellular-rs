

use crate::{Config, GsmClient};

#[derive(Debug, defmt::Format)]
pub struct State<S, RST, DTR, PWR, VINT> {
    config: Config<RST, DTR, PWR, VINT>,
    // client: GsmClient<>,
    _inner: S
}

impl<RST, DTR, PWR, VINT> State<Init, RST, DTR, PWR, VINT> {
    pub fn new(config: Config<RST, DTR, PWR, VINT>) -> Self {
        State {
            config,

            _inner: Init
        }
    }



}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct Init;

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct PowerOn {

}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct DeviceReady {

}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct SimPin {

}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct SignalQuality {

}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct RegisteringNetwork {

}

#[derive(Debug, Clone, Copy, PartialEq, defmt::Format)]
pub struct AttachingNetwork {

}


fn test() {
    let gsm = State::new(Config::default());


}
