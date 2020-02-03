use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    AT,
    ATS,
    Flow,
    Baud,
    /// 4.7 IMEI identification +CGSN
    /// Returns the product serial number, the International Mobile Equipment Identity (IMEI) of the MT.
    GetIMEI,
    /// 4.12 Card identification +CCID
    /// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
    GetCCID,

    /// 5.3 Set module functionality +CFUN
    SetModuleFunctionality {
        fun: Functionality,
        rst: Option<ResetMode>,
    },
    /// 5.19 Report mobile termination error +CMEE
    /// Configures the formatting of the result code +CME ERROR: <err> as an indication of an error relating to the
    /// functionality of the MT. When enabled, MT related errors cause +CME ERROR: <err> final result code instead
    /// of the regular ERROR final result code. The error result code is returned normally when an error is related to
    /// syntax, invalid parameters or MT functionality
    SetReportMobileTerminationError {
        n: TerminationErrorMode,
    },

    /// 18.7 Packet switched data configuration +UPSD
    /// Sets or reads all the parameters in a specific packet switched data (PSD) profile. The command is used to set
    /// up the PDP context parameters for an internal context, i.e. a data connection using the internal IP stack and
    /// related AT commands for sockets.
    /// To set all the parameters of the PSD profile a set command for each parameter needs to be issued.
    SetPacketSwitchedConfig {
        profile_id: u8,
        param: PacketSwitchedParam,
    },
    GetPacketSwitchedConfig {
        profile_id: u8,
    },
    /// 18.8 Packet switched data action +UPSDA
    /// Performs the requested action for the specified PSD profile.
    /// The command can be aborted. When a PDP context activation (<action>=3) or a PDP context deactivation
    /// (<action>=4) is aborted, the +UUPSDA URC is provided. The <result> parameter indicates the operation result.
    /// Until this operation is not completed, another set command cannot be issued.
    /// The +UUPSDD URC is raised when the data connection related to the provided PSD profile is deactivated either
    /// explicitly by the network (e.g. due to prolonged idle time) or locally by the module after a failed PS registration
    /// procedure (e.g. due to roaming) or a user required detach (e.g. triggered by AT+COPS=2).
    SetPacketSwitchedAction {
        profile_id: u8,
        action: PacketSwitchedAction
    },
    /// 18.9 Packet switched network-assigned data +UPSND
    /// Returns the current (dynamic) network-assigned or network-negotiated value of the specified parameter for
    /// the active PDP context associated with the specified PSD profile.
    GetPacketSwitchedNetworkData {
        profile_id: u8,
        param: PacketSwitchedNetworkDataParam
    },
    /// 18.14 GPRS attach or detach +CGATT
    /// Register (attach) the MT to, or deregister (detach) the MT from the GPRS service. After this command the MT
    /// remains in AT command mode. If the MT is already in the requested state (attached or detached), the command
    /// is ignored and OK result code is returned. If the requested state cannot be reached, an error result code is
    /// returned. The command can be aborted if a character is sent to the DCE during the command execution. Any
    /// active PDP context will be automatically deactivated when the GPRS registration state changes to detached.
    SetGPRSAttached {
        state: bool
    },
    /// 19.8 Power saving control (Power SaVing) +UPSV
    SetPowerSavingControl {
        mode: PowerSavingMode,
        timeout: Option<Seconds>,
    },

    /// 20.2 GPIO select configuration command +UGPIOC
    SetGpioConfiguration {
        /// GPIO pin identifier: pin number
        /// See the GPIO mapping for the available GPIO pins, their mapping and factoryprogrammed values on different u-blox cellular modules series and product version.
        gpio_id: u8,
        /// Mode identifier: configured function
        /// See the GPIO functions for custom functions supported by different u-blox cellular
        /// modules series and product version
        gpio_mode: GpioMode,
    },

    /// 25.3 Create Socket +USOCR
    /// Creates a socket and associates it with the specified protocol (TCP or UDP), returns a number identifying the
    /// socket. Such command corresponds to the BSD socket routine:
    /// • TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 / SARA-G4 / SARA-G340 /
    /// SARA-G350 - Up to 7 sockets can be created.
    /// • LEON-G1 - Up to 16 sockets can be created
    /// It is possible to specify the local port to bind within the socket in order to send data from a specific port. The
    /// bind functionality is supported for both TCP and UDP sockets.
    CreateSocket {
        protocol: SocketProtocol
    },
}

// /// 5.3 Set module functionality +CFUN
// CMD!(SetModuleFunctionality, "AT+CFUN={}")

// pub trait ATCommand {
//     type Response;

//     fn cmd(&self) -> &str;

//     fn can_abort(&self) -> bool {
//         false
//     }

//     fn max_timeout_ms(&self) -> u32 {
//         1000
//     }

//     fn response(
//         &self,
//         lines: &str,
//     ) -> Self::Response;
// }

// pub mod testcommands {
//     use super::*;

//     /// 4.12 Card identification +CCID
//     /// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
//     pub struct GetCCID;

//     impl ATCommand for GetCCID {
//         type Response = Option<()>;

//         fn cmd<N: ArrayLength<u8>>(&self) -> String<N> {
//             String::from("AT+CCID")
//         }

//         fn response<N: ArrayLength<u8>, L: ArrayLength<String<N>>>(
//             &self,
//             lines: Vec<String<N>, L>,
//         ) -> Self::Response {
//             None
//         }
//     }

//     /// 5.3 Set module functionality +CFUN
//     pub struct SetModuleFunctionality {
//         pub fun: Functionality,
//         pub rst: Option<ResetMode>,
//     }

//     impl ATCommand for SetModuleFunctionality {
//         type Response = Option<()>;

//         fn cmd<N: ArrayLength<u8>>(&self) -> String<N> {
//             let mut b = String::new();
//             write!(b, "AT+CFUN={}", self.fun as u8).unwrap();
//             if let Some(rst) = self.rst {
//                 write!(b, ",{}", rst as u8).unwrap();
//             }
//             b
//         }

//         fn response<N: ArrayLength<u8>, L: ArrayLength<String<N>>>(
//             &self,
//             lines: Vec<String<N>, L>,
//         ) -> Self::Response {
//             None
//         }

//         fn max_timeout_ms(&self) -> u32 {
//             1_000_000
//         }

//         fn can_abort(&self) -> bool {
//             true
//         }
//     }
// }

// fn foo() -> impl ATCommand {
//     commands::GetCCID
// }
// fn bar() -> impl ATCommand {
//     commands::SetModuleFunctionality {
//         fun: Functionality::Full,
//         rst: None,
//     }
// }

// fn test() {
//     let f1 = bar();
//     let f2 = commands::GetCCID;
//     let f3 = commands::SetModuleFunctionality {
//         fun: Functionality::Full,
//         rst: None,
//     };
//     f1.max_timeout_ms();
//     f2.max_timeout_ms();
//     f3.max_timeout_ms();
// }


// # RFC: Refactor command structure
// I am opening this issue as an invitation to discuss the way this crate expects implementors to structure AT commands.

// Just as a sum up, the current way expects commands to look something along the lines of and enum with commands as,

// ```
// use core::fmt::Write;
// use heapless::{ArrayLength, String, Vec};

// use at::{utils, ATCommandInterface, ATRequestType, MaxCommandLen, MaxResponseLines};

// #[derive(Debug, Clone, PartialEq)]
// pub enum Command {
//     AT,
//     GetManufacturerId,
//     GetModelId,
//     GetFWVersion,
//     GetSerialNum,
//     GetId,
//     SetEcho { enable: bool },
//     GetEcho,
// }

// #[allow(dead_code)]
// #[derive(Debug)]
// pub enum Response {
//     ManufacturerId {
//         id: String<MaxCommandLen>,
//     },
//     ModelId {
//         id: String<MaxCommandLen>,
//     },
//     FWVersion {
//         version: String<MaxCommandLen>,
//     },
//     SerialNum {
//         serial: String<MaxCommandLen>,
//     },
//     Id {
//         id: String<MaxCommandLen>,
//     },
//     Echo {
//         enable: bool,
//     },
//     None,
// }

// impl ATRequestType for Command {
//     type Command = Command;

//     fn try_get_cmd(self) -> Option<Self::Command> {
//         Some(self)
//     }

//     fn get_bytes<N: ArrayLength<u8>>(&self) -> Vec<u8, N> {
//         self.get_cmd().into_bytes()
//     }
// }

// impl ATCommandInterface for Command {
//     type Response = Response;

//     fn get_cmd<N: ArrayLength<u8>>(&self) -> String<N> {
//         let mut buffer = String::new();
//         match self {
//             Command::AT => String::from("AT"),
//             Command::GetManufacturerId => String::from("AT+CGMI"),
//             Command::GetModelId => String::from("AT+CGMM"),
//             Command::GetFWVersion => String::from("AT+CGMR"),
//             Command::GetSerialNum => String::from("AT+CGSN"),
//             Command::GetId => String::from("ATI9"),
//             Command::SetEcho { ref enable } => {
//                 write!(buffer, "ATE{}", *enable as u8).unwrap();
//                 buffer
//             }
//             Command::GetEcho => String::from("ATE?"),
//         }
//     }

//     fn parse_resp(
//         &self,
//         response_lines: &Vec<String<MaxCommandLen>, MaxResponseLines>,
//     ) -> Response {
//         if response_lines.is_empty() {
//             return Response::None;
//         }
//         let mut responses: Vec<Vec<&str, MaxResponseLines>, MaxResponseLines> =
//             utils::split_parameterized_resp(response_lines);

//         let response = responses.pop().unwrap();

//         match *self {
//             Command::AT => Response::None,
//             Command::GetManufacturerId => Response::ManufacturerId {
//                 id: String::from(response[0]),
//             },
//             _ => Response::None,
//         }
//     }

//     fn parse_unsolicited(_response_line: &str) -> Option<Response> {
//         Some(Response::None)
//     }
// }

// ```

// But this makes a couple of things very difficult:
// 1. It makes doing macro generation of AT commands very difficult, due to the big coupled enum.
// 2. It means a number of rather large match cases for `get_cmd` & `parse_resp`, and would require the same match cases in order to implement eg. `max_timeout_time`, `can_abort` etc.

// So, because of this my suggestion would be to restructure this into something along the lines of a `ATCommand` trait in this crate:

// ```
// pub trait ATCommand {
//     type Response;

//     fn cmd<N: ArrayLength<u8>>(&self) -> String<N>;

//     fn can_abort(&self) -> bool {
//         false
//     }

//     fn max_timeout_ms(&self) -> u32 {
//         1000
//     }

//     fn response<N: ArrayLength<u8>, L: ArrayLength<String<N>>>(&self, lines: Vec<String<N>, L>) -> Self::Response;
// }
// ```

// This would allow implementors to use it as:

// ```
// mod commands {
//     use super::*;

//     /// 4.12 Card identification +CCID
//     /// Returns the ICCID (Integrated Circuit Card ID) of the SIM-card. ICCID is a serial number identifying the SIM.
//     pub struct GetCCID;

//     impl ATCommand for GetCCID {
//         type Response = Option<()>;

//         fn cmd<N: ArrayLength<u8>>(&self) -> String<N> {
//             String::from("AT+CCID")
//         }

//         fn response<N: ArrayLength<u8>, L: ArrayLength<String<N>>>(
//             &self,
//             lines: Vec<String<N>, L>,
//         ) -> Self::Response {
//             None
//         }
//     }

//     /// 5.3 Set module functionality +CFUN
//     pub struct SetModuleFunctionality {
//         pub fun: Functionality,
//         pub rst: Option<ResetMode>,
//     }

//     impl ATCommand for SetModuleFunctionality {
//         type Response = Option<()>;

//         fn cmd<N: ArrayLength<u8>>(&self) -> String<N> {
//             let mut b = String::new();
//             write!(b, "AT+CFUN={}", self.fun as u8).unwrap();
//             if let Some(rst) = self.rst {
//                 write!(b, ",{}", rst as u8).unwrap();
//             }
//             b
//         }

//         fn response<N: ArrayLength<u8>, L: ArrayLength<String<N>>>(
//             &self,
//             lines: Vec<String<N>, L>,
//         ) -> Self::Response {
//             None
//         }

//         fn max_timeout_ms(&self) -> u32 {
//             1_000_000
//         }

//         fn can_abort(&self) -> bool {
//             true
//         }
//     }
// }
// ```

// Making it much easier to create macros/proc_macros to implement these in a derive crate, while giving back implementors better handling of parsing responses etc.
// Also it might make it easier to add more features to the commands (I have added the `max_timeout_ms` and the `can_abort` here, both with a default implementation.

// Furthermore because of the
