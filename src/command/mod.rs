//! AT Commands for u-blox cellular module family\
//! Following the [u-blox cellular modules AT commands manual](https://www.u-blox.com/sites/default/files/u-blox-CEL_ATCommands_%28UBX-13002752%29.pdf)

pub mod control;
pub mod device_data_security;
pub mod device_lock;
pub mod dns;
pub mod file_system;
pub mod general;
pub mod gpio;
pub mod http;
pub mod ip_transport_layer;
pub mod mobile_control;
pub mod network_service;
pub mod psn;
pub mod sms;
pub mod system_features;

use atat::atat_derive::{AtatCmd, AtatResp, AtatUrc};

#[derive(Clone, AtatResp)]
pub struct NoResponse;

#[derive(Clone, AtatCmd)]
#[at_cmd("", NoResponse)]
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

    #[at_urc("+UUSORD")]
    SocketDataAvailable(ip_transport_layer::urc::SocketDataAvailable),
    #[at_urc("+UUSORF")]
    SocketDataAvailableUDP(ip_transport_layer::urc::SocketDataAvailable),
    #[at_urc("+UUPSDA")]
    DataConnectionActivated(psn::urc::DataConnectionActivated),
    #[at_urc("+UUPSDD")]
    DataConnectionDeactivated(psn::urc::DataConnectionDeactivated),
    #[at_urc("+UUSOCL")]
    SocketClosed(ip_transport_layer::urc::SocketClosed),

    #[at_urc("+UMWI")]
    MessageWaitingIndication(sms::urc::MessageWaitingIndication),
    // #[at_urc("+CREG")]
    // NetworkRegistration(network_service::urc::NetworkRegistration),
    // #[at_urc("+CGREG")]
    // GPRSNetworkRegistration(psn::urc::GPRSNetworkRegistration),
    // #[at_urc("+CEREG")]
    // EPSNetworkRegistration(psn::urc::EPSNetworkRegistration),
    #[at_urc("+UREG")]
    ExtendedPSNetworkRegistration(psn::urc::ExtendedPSNetworkRegistration),

    #[at_urc("+UUHTTPCR")]
    HttpResponse(http::urc::HttpResponse),
}
