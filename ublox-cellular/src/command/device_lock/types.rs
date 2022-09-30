//! Argument and parameter types used by Device lock Commands and Responses

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinStatusCode {
    /// • READY: MT is not pending for any password
    Ready,
    /// • SIM PIN: MT is waiting SIM PIN to be given
    SimPin,
    /// • SIM PUK: MT is waiting SIM PUK to be given
    SimPuk,
    /// • SIM PIN2: MT is waiting SIM PIN2 to be given
    SimPin2,
    /// • SIM PUK2: MT is waiting SIM PUK2 to be given
    SimPuk2,
    /// • PH-NET PIN: MT is waiting network personalization password to be given
    PhNetPin,
    /// • PH-NETSUB PIN: MT is waiting network subset personalization password to be
    /// given
    PhNetSubPin,
    /// • PH-SP PIN: MT is waiting service provider personalization password to be given
    PhSpPin,
    /// • PH-CORP PIN: MT is waiting corporate personalization password to be given
    PhCorpPin,
    /// • PH-SIM PIN: MT is waiting phone to SIM/UICC card password to be given
    PhSimPin,
}
