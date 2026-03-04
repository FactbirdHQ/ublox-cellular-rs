//! 29 HTTP The section describes the u-blox proprietary AT commands that can be
//! used for sending requests to a remote HTTP server, receiving the server
//! response and transparently storing it in the file system. The supported
//! methods are: HEAD, GET, DELETE, PUT, POST file and POST data. A PSD or CSD
//! connection must be activated before using HTTP AT commands.
//!
//! **NOTES:**
//! - **TOBY-L2 / MPCI-L2 / LARA-R2 / TOBY-R2 / SARA-U2 / LISA-U2 / LISA-U1 /
//!   SARA-G4 / SARA-G3 / LEON-G1** - See +UPSD, +UPSDA and +UPSND AT commands
//!   for establishing a PSD connection.
//! - **SARA-G3 / LEON-G1** - See +UCSD, +UCSDA and +UCSND AT commands for
//!   establishing a CSD connection.
//!
//! When these commands report an HTTP error, the error code can be queried
//! using the +UHTTPER AT command.
//!
//! - **LISA-U200-00S / LISA-U200-01S / LISA-U200-02S / LISA-U200-52S /
//!   LISA-U200-62S / LISA-U230 / LISA-U260 / LISA-U270 / LISA-U1 / LEON-G1* -
//!   If using `CellLocate`® and HTTP commands HTTP profiles in the range 1-3 must
//!   be used.

pub mod urc;
