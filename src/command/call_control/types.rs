//! Argument and parameter types used by Call Control Commands and Responses

use atat::atat_derive::AtatEnum;

/// Type of address in integer format
#[derive(Clone, Default, PartialEq, Eq, AtatEnum)]
pub enum AddressType {
    /// 145: dialing string includes international access code character '+'
    IncludeNationalAccessCode = 145,

    /// 129 (default value): national coded dialing string
    #[default]
    NationalCodedString = 129,
}
