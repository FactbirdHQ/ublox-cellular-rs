//! AT Commands for U-Blox short range module family\
//! Following [ATCommands Manual](https://www.u-blox.com/sites/default/files/u-connect-ATCommands-Manual_(UBX-14044127).pdf)

use core::fmt::Write;
use heapless::{ArrayLength, String, Vec, consts};

use at::{ATCommandInterface, ATRequestType};

use embedded_nal::{IpAddress, Port};

mod cmd;

mod response;
mod types;

pub use cmd::Command;
use log::{info, warn};
pub use response::{Response, UnsolicitedResponse};
pub use types::*;
use crate::socket::SocketHandle;

#[derive(Debug, Clone)]
pub enum ResponseType {
    SingleSolicited(Response),
    Unsolicited(UnsolicitedResponse),
    None,
}

#[derive(Debug, Clone)]
pub enum RequestType {
    Cmd(Command),
}

impl ATRequestType for RequestType {
    type Command = Command;

    fn try_get_cmd(self) -> Option<Self::Command> {
        match self {
            RequestType::Cmd(c) => Some(c),
            _ => None,
        }
    }

    fn get_bytes<N: ArrayLength<u8>>(&self) -> Vec<u8, N> {
        match self {
            RequestType::Cmd(c) => {
                let mut command = c.get_cmd();
                if !command.ends_with("\r\n") {
                    command.push_str("\r\n").ok();
                }
                command.into_bytes()
            }
        }
    }
}

impl ATCommandInterface for Command {
    type Response = ResponseType;

    fn get_cmd<N: ArrayLength<u8>>(&self) -> String<N> {
        let mut buffer = String::new();
        match self {
            Command::AT => String::from("AT"),
            Command::Flow => String::from("AT&K3"),
            Command::Baud => String::from("AT+IPR=115200"),
            Command::ATS => String::from("ATS4?"),
            Command::GetIMEI => String::from("AT+CGSN"),
            Command::GetCCID => String::from("AT+CCID"),
            Command::SetReportMobileTerminationError { n } => {
                write!(buffer, "AT+CMEE={}", n.clone() as u8).unwrap();
                buffer
            }
            Command::SetModuleFunctionality { fun, rst } => {
                write!(buffer, "AT+CFUN={}", fun.clone() as u8).unwrap();
                if let Some(rst) = rst {
                    write!(buffer, ",{}", rst.clone() as u8).unwrap();
                }
                buffer
            }
            Command::SetPacketSwitchedConfig {
                ref profile_id,
                ref param,
            } => {
                write!(buffer, "AT+UPSD={}", *profile_id as u8).unwrap();
                match param {
                    PacketSwitchedParam::ProtocolType(p) => {
                        write!(buffer, ",0,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::APN(s) => write!(buffer, ",1,{:?}", s).unwrap(),
                    PacketSwitchedParam::Username(s) => write!(buffer, ",2,{:?}", s).unwrap(),
                    PacketSwitchedParam::Password(s) => write!(buffer, ",3,{:?}", s).unwrap(),
                    PacketSwitchedParam::DNS1(ip) => write!(buffer, ",4,{}", ip).unwrap(),
                    PacketSwitchedParam::DNS2(ip) => write!(buffer, ",5,{}", ip).unwrap(),
                    PacketSwitchedParam::Authentication(p) => {
                        write!(buffer, ",6,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::IPAddress(ip) => write!(buffer, ",7,{}", ip).unwrap(),
                    PacketSwitchedParam::DataCompression(p) => {
                        write!(buffer, ",8,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::HeaderCompression(p) => {
                        write!(buffer, ",9,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::QoSPrecedence(p) => {
                        write!(buffer, ",10,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::QoSDelay(p) => {
                        write!(buffer, ",11,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::QoSReliability(p) => {
                        write!(buffer, ",12,{}", p.clone() as u8).unwrap()
                    }
                    PacketSwitchedParam::QoSDelay3G(u) => write!(buffer, ",51,{}", u).unwrap(),
                    PacketSwitchedParam::CurrentProfileMap(u) => {
                        write!(buffer, ",100,{}", u).unwrap()
                    }
                };
                buffer
            }
            Command::GetPacketSwitchedConfig { ref profile_id } => {
                write!(buffer, "AT+UPSD={}", *profile_id as u8).unwrap();
                // match param {
                //     PacketSwitchedParam::ProtocolType(p) => write!(buffer, ",0,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::APN(s) => write!(buffer, ",1,{:?}", s).unwrap(),
                //     PacketSwitchedParam::Username(s) => write!(buffer, ",2,{:?}", s).unwrap(),
                //     PacketSwitchedParam::Password(s) => write!(buffer, ",3,{:?}", s).unwrap(),
                //     PacketSwitchedParam::DNS1(ip) => write!(buffer, ",4,{}", ip).unwrap(),
                //     PacketSwitchedParam::DNS2(ip) => write!(buffer, ",5,{}", ip).unwrap(),
                //     PacketSwitchedParam::Authentication(p) => write!(buffer, ",6,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::IPAddress(ip) => write!(buffer, ",7,{}", ip).unwrap(),
                //     PacketSwitchedParam::DataCompression(p) => write!(buffer, ",8,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::HeaderCompression(p) => write!(buffer, ",9,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::QoSPrecedence(p) => write!(buffer, ",10,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::QoSDelay(p) => write!(buffer, ",11,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::QoSReliability(p) => write!(buffer, ",12,{}", p.clone() as u8).unwrap(),
                //     PacketSwitchedParam::QoSDelay3G(u) => write!(buffer, ",51,{}", u).unwrap(),
                //     PacketSwitchedParam::CurrentProfileMap(u) => write!(buffer, ",100,{}", u).unwrap(),
                // };
                buffer
            }
            Command::SetPacketSwitchedAction {
                ref profile_id,
                ref action,
            } => {
                write!(
                    buffer,
                    "AT+UPSDA={},{}",
                    *profile_id as u8,
                    action.clone() as u8
                )
                .unwrap();
                buffer
            }
            Command::GetPacketSwitchedNetworkData {
                ref profile_id,
                ref param,
            } => {
                write!(
                    buffer,
                    "AT+UPSND={},{}",
                    *profile_id as u8,
                    param.clone() as u8
                )
                .unwrap();
                buffer
            }
            Command::SetGPRSAttached { state } => {
                write!(buffer, "AT+CGATT={}", *state as u8).unwrap();
                buffer
            }
            Command::SetPowerSavingControl { mode, timeout } => {
                write!(buffer, "AT+UPSV={}", mode.clone() as u8).unwrap();
                if let Some(timeout) = timeout {
                    write!(buffer, ",{}", timeout).unwrap();
                }
                buffer
            }
            Command::SetGpioConfiguration { gpio_id, gpio_mode } => {
                write!(buffer, "AT+UGPIOC={},{}", gpio_id, gpio_mode).unwrap();
                match gpio_mode {
                    GpioMode::Output(v) => write!(buffer, ",{}", v.clone() as u8).unwrap(),
                    GpioMode::Input(p) => write!(buffer, ",{}", p.clone() as u8).unwrap(),
                    _ => (),
                };
                buffer
            }
            Command::CreateSocket { protocol } => {
                write!(buffer, "AT+USOCR={}", protocol.clone() as u8).unwrap();
                buffer
            }
            Command::CloseSocket { socket } => {
                write!(buffer, "AT+USOCL={}", socket).unwrap();
                buffer
            }
            Command::GetSocketError => String::from("AT+USOER"),
            Command::ConnectSocket {
                socket,
                remote_addr,
                remote_port,
            } => {
                let Port(port) = remote_port;
                match remote_addr {
                    IpAddress::IpV4(ipv4) => {
                        write!(buffer, "AT+USOCO={},\"{}\",{}", socket, ipv4, port).unwrap()
                    }
                    IpAddress::IpV6(ipv6) => {
                        write!(buffer, "AT+USOCO={},\"{}\",{}", socket, ipv6, port).unwrap()
                    }
                };
                buffer
            }
            Command::WriteSocketData {
                socket,
                length,
                data
            } => {
                // TODO: Do this without clones!
                let s = String::from_utf8(data.clone()).unwrap();
                write!(buffer, "AT+USOWR={},{},\"{}\"", socket, length, s).unwrap();
                buffer
            }
            Command::ReadSocketData {
                socket,
                length,
            } => {
                write!(buffer, "AT+USORD={},{}", socket, length).unwrap();
                buffer
            }
            _ => String::from(""),
        }
    }

    fn parse_resp(
        &self,
        response_lines: &Vec<String<at::MaxCommandLen>, at::MaxResponseLines>,
    ) -> ResponseType {
        if response_lines.is_empty() {
            return ResponseType::None;
        }

        // Handle list items
        // let mut responses = at::utils::split_parameterized_resp(response_lines);

        match *self {
            // Command::AT => ResponseType::None,
            _ => {
                warn!("Unimplemented response for cmd {:?}\r", self);
                ResponseType::None
            }
        }
    }

    fn parse_unsolicited(response_line: &str) -> Option<ResponseType> {
        let (cmd, response) = at::utils::split_parameterized_unsolicited(response_line);
        info!("Unsolicited {:?} - {:?}\r", cmd, response);
        Some(match cmd {
            "+UMWI" => ResponseType::None,
            "+UUPSDD" => ResponseType::None,
            "+UUSOCL" => ResponseType::None,
            "+UUSORD" => ResponseType::None,
            _ => return None,
        })
    }
}
