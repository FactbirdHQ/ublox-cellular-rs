//! AT Commands for u-blox cellular module family\
//! Following the [u-blox cellular modules AT commands manual](https://content.u-blox.com/sites/default/files/u-blox-CEL_ATCommands_UBX-13002752.pdf)

pub mod call_control;
pub mod control;
pub mod device_data_security;
pub mod device_lock;
pub mod dns;
pub mod file_system;
pub mod general;
pub mod gpio;
pub mod http;
#[cfg(feature = "internal-network-stack")]
pub mod ip_transport_layer;
pub mod ipc;
pub mod mobile_control;
pub mod network_service;
pub mod networking;
pub mod psn;
pub mod sms;
pub mod system_features;

use atat::{
    atat_derive::{AtatCmd, AtatResp, AtatUrc},
    nom,
};

#[derive(Clone, AtatResp)]
pub struct NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse, attempts = 3)]
pub struct AT;

#[derive(Debug, Clone, AtatUrc)]
pub enum Urc {
    #[at_urc("+CGEV: NW DETACH")]
    NetworkDetach,
    #[at_urc("+CGEV: ME DETACH")]
    MobileStationDetach,
    #[at_urc("+CGEV: NW DEACT")]
    NetworkDeactivate,
    #[at_urc("+CGEV: ME DEACT")]
    MobileStationDeactivate,
    #[at_urc("+CGEV: NW PDN DEACT")]
    NetworkPDNDeactivate,
    #[at_urc("+CGEV: ME PDN DEACT")]
    MobileStationPDNDeactivate,

    #[cfg(feature = "internal-network-stack")]
    #[at_urc("+UUSORD")]
    SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
    #[cfg(feature = "internal-network-stack")]
    #[at_urc("+UUSORF")]
    SocketDataAvailableUDP(ip_transport_layer::urc::SocketDataAvailable),
    #[cfg(feature = "internal-network-stack")]
    #[at_urc("+UUSOCL")]
    SocketClosed(ip_transport_layer::urc::SocketClosed),

    #[at_urc("+UUPSDA")]
    DataConnectionActivated(psn::urc::DataConnectionActivated),
    #[at_urc("+UUPSDD")]
    DataConnectionDeactivated(psn::urc::DataConnectionDeactivated),

    #[at_urc("+UMWI")]
    MessageWaitingIndication(sms::urc::MessageWaitingIndication),
    #[at_urc("+CREG", parse = custom_cxreg_parse)]
    NetworkRegistration(network_service::urc::NetworkRegistration),
    #[at_urc("+CGREG", parse = custom_cxreg_parse)]
    GPRSNetworkRegistration(psn::urc::GPRSNetworkRegistration),
    #[at_urc("+CEREG", parse = custom_cxreg_parse)]
    EPSNetworkRegistration(psn::urc::EPSNetworkRegistration),
    #[at_urc("+UREG")]
    ExtendedPSNetworkRegistration(psn::urc::ExtendedPSNetworkRegistration),

    #[at_urc("+UUHTTPCR")]
    HttpResponse(http::urc::HttpResponse),
}

fn custom_cxreg_parse<'a, T, Error: nom::error::ParseError<&'a [u8]> + core::fmt::Debug>(
    token: T,
) -> impl Fn(&'a [u8]) -> nom::IResult<&'a [u8], (&'a [u8], usize), Error>
where
    &'a [u8]: nom::Compare<T> + nom::FindSubstring<T>,
    T: nom::InputLength + Clone + nom::InputTake + nom::InputIter + nom::AsBytes,
{
    move |i| {
        let (i, (urc, len)) = atat::digest::parser::urc_helper(token.clone())(i)?;

        let index = urc.iter().position(|&x| x == b':').unwrap_or(urc.len());
        let arguments = &urc[index + 1..];

        // "+CxREG?" response will always have atleast 2 arguments, both being
        // integers.
        //
        // "+CxREG:" URC will always have at least 1 integer argument, and the
        // second argument, if present, will be a string.

        // Parse the first
        let (rem, _) = nom::sequence::tuple((
            nom::character::complete::space0,
            nom::number::complete::u8,
            nom::branch::alt((nom::combinator::eof, nom::bytes::complete::tag(","))),
        ))(arguments)?;

        if !rem.is_empty() {
            // If we have more arguments, we want to make sure this is a quoted string for the URC case.
            nom::sequence::tuple((
                nom::character::complete::space0,
                nom::sequence::delimited(
                    nom::bytes::complete::tag("\""),
                    nom::bytes::complete::escaped(
                        nom::character::streaming::none_of("\"\\"),
                        '\\',
                        nom::character::complete::one_of("\"\\"),
                    ),
                    nom::bytes::complete::tag("\""),
                ),
                nom::branch::alt((nom::combinator::eof, nom::bytes::complete::tag(","))),
            ))(rem)?;
        }

        Ok((i, (urc, len)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_parse_cxreg() {
        let creg_resp = b"\r\n+CREG: 2,5,\"9E9A\",\"019624BD\",2\r\n";
        let creg_urc_min = b"\r\n+CREG: 0\r\n";
        let creg_urc_full = b"\r\n+CREG: 5,\"9E9A\",\"0196BDB0\",2\r\n";

        assert!(
            custom_cxreg_parse::<&[u8], nom::error::Error<&[u8]>>(&b"+CREG"[..])(creg_resp)
                .is_err()
        );
        assert!(
            custom_cxreg_parse::<&[u8], nom::error::Error<&[u8]>>(&b"+CREG"[..])(creg_urc_min)
                .is_ok()
        );
        assert!(
            custom_cxreg_parse::<&[u8], nom::error::Error<&[u8]>>(&b"+CREG"[..])(creg_urc_full)
                .is_ok()
        );
    }
}
