//! ### 22 - File System
//!
//! File system commands have the optional <tag> parameter that allows the user to specify a file type when a
//! file system AT command is issued, to inform the system what to do with it. Application specific files must be
//! saved with the correct type tag, otherwise they are treated as common user files.
pub mod responses;

use atat::atat_derive::AtatCmd;
use heapless::{String, Vec};
use responses::*;

use super::NoResponse;

/// 22.2 Download file +UDWNFILE
///
/// Stores (writes) a file into the file system:
/// - The stream of bytes can be entered after the '>' prompt has been provided
///   to the user. The file transfer is terminated exactly when <size> bytes
///   have been entered and either "OK" final result code or an error result
///   code is returned. The feed process cannot be interrupted i.e. the command
///   mode is re-entered once the user has provided the declared the number of
///   bytes.
/// - If the file already exists, the data will be appended to the file already
///   stored in the file system.
/// - If the data transfer stops, after 20 s the command is stopped and the
///   "+CME ERROR: FFS TIMEOUT" error result code (if +CMEE: 2) is returned.
/// - If the module shuts down during the file storing, all bytes of the file
///   will be deleted.
/// - If an error occurs during the file writing, the transfer is aborted and it
///   is up to the user to delete the file.
///
/// **Notes:**
/// - **TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 /
///   LISA-U1 / SARA-G4 / SARA-G3 / LEON-G1** - The available free memory space
///   is checked before starting the file transfer. If the file size exceeds the
///   available space, the "+CME ERROR: NOT ENOUGH FREE SPACE" error result code
///   will be provided (if +CMEE: 2).
/// - **TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 /
///   SARA-G4 / SARA-G3 / LEON-G1** - If the HW flow control is disabled
///   (AT&K0), a data loss could be experienced. So the HW flow control usage is
///   strongly recommended.
/// - **TOBY-L4** - The '>' prompt after which the stream of bytes can be
///   entered will be provided to the user on a dedicated channel of the USB
///   CDC-ACM interface. If the command is issued on the AT interface over an IP
///   connection, the DTE will send the binary data over the TCP connection to
///   the DCE. The DTE monitors the TCP data port for the binary data transfer
///   (for more details on the TCP/IP port configuration, see the
///   <tcp_data_port> parameter of the +UIFCONF AT command). After the
///   establishment of the TCP connection from the DTE to the specific port, the
///   DTE will start the file transfer. The '>' prompt is not provided to the
///   user on the AT interface over an IP connection. The DCE will close the
///   connection when the specific amount of data is received, or an error
///   result code occurs. Once the AT command is issued, the DCE will listen on
///   the specific port and will close it after the timeout expiration (20 s).
///   The DCE will close the TCP connection, if no data are received for 30 s.
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDWNFILE", NoResponse)]
pub struct PrepareDownloadFile<'a> {
    #[at_arg(position = 0, len = 248)]
    pub filename: &'a str,
    #[at_arg(position = 1)]
    pub size: usize,
}

#[derive(Clone, AtatCmd)]
#[at_cmd(
    "",
    NoResponse,
    value_sep = false,
    cmd_prefix = "",
    termination = "",
    force_receive_state = true
)]
pub struct DownloadFile<'a> {
    #[at_arg(position = 0, len = 2048)]
    pub text: &'a atat::serde_bytes::Bytes,
}

/// 22.3 List files information +ULSTFILE
///
/// Retrieves some information about the FS. Depending on the specified
/// <op_code>, it can print:
/// - List of files stored into the FS
/// - Remaining free FS space expressed in bytes
/// - Size of the specified file expressed in bytes
///
/// **NOTES:** The available free space on FS in bytes reported by the command
/// AT+ULSTFILE=1 is the theoretical free space including the space occupied by
/// the hidden and temporary files which are not displayed by the AT+ULSTFILE=0.
#[derive(Clone, AtatCmd)]
#[at_cmd("+ULSTFILE=0", Vec<String<248>, 10>, value_sep = false)]
pub struct ListFiles;

/// 22.4 Read file +URDFILE
///
/// Retrieves a file from the file system.
///
/// **NOTES:**
/// - **TOBY-L4** - The stream of file bytes will be provided to the user on a
///   dedicated channel of the USB CDC-ACM interface. If the command is issued
///   on the AT interface over an IP connection, the DCE will send the binary
///   data over the TCP connection to the DTE. The DTE monitors the TCP data
///   port for the binary data transfer (for more details on the TCP/IP port
///   configuration, see the <tcp_data_port> parameter of the +UIFCONF AT
///   command). After the establishment of the TCP connection from the DTE to
///   the specific port, the DCE starts the file transfer. The '>' prompt is not
///   provided to the user on the AT interface over an IP connection. The DCE
///   will close the connection when the specific amount of data is transmitted,
///   or an error result code occurs. Once the AT command is issued, the DCE
///   will listen on the specific port and will close it after the timeout
///   expiration (20 s). The DCE will close the TCP connection, if no data are
///   received for 30 s.
/// - **LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1** - During the command
///   execution, if the DTE stops reading any data from the module (using flow
///   control mechanisms) for 5 s or more, the command could be aborted by the
///   module, and an error result code is returned. In case the DTE is not able
///   to sustain the data flow and to avoid the HW flow control intervention for
///   long time, the use of the +URDBLOCK AT command is recommended.
#[derive(Clone, AtatCmd)]
#[at_cmd("+URDFILE", ReadFileResponse)]
pub struct ReadFile<'a> {
    #[at_arg(position = 0, len = 248)]
    pub filename: &'a str,
}

/// 22.5 Partial read file +URDBLOCK
///
/// Retrieves a file from the file system.
///
/// **NOTES:** -Differently from +URDFILE command, this command allows the user
/// to read only a portion of the file, indicating the offset and amount of
/// bytes.
#[derive(Clone, AtatCmd)]
#[at_cmd("+URDBLOCK", ReadBlockResponse)]
pub struct ReadBlock<'a> {
    #[at_arg(position = 0, len = 248)]
    pub filename: &'a str,
    #[at_arg(position = 1)]
    pub offset: usize,
    #[at_arg(position = 2)]
    pub size: usize,
}

/// 22.6 Delete file +UDELFILE
///
/// Deletes a stored file from the file system.
///
/// **NOTES:**
/// - **TOBY-L4 / TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 /
///   LISA-U1 / SARA-G3 / LEON-G1** - If <filename> file is not stored in the
///   file system the following error result code will be provided: "+CME ERROR:
///   FILE NOT FOUND".
/// - **SARA-G4** - If <filename> file is not stored in the file system the
///   following error result code will be provided: "+CME ERROR: FFS error".
#[derive(Clone, AtatCmd)]
#[at_cmd("+UDELFILE", NoResponse)]
pub struct DeleteFile<'a> {
    #[at_arg(position = 0, len = 248)]
    pub filename: &'a str,
}
