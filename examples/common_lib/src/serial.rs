use embedded_hal as hal;
use serialport;
use std::io::{ErrorKind as IoErrorKind, Read, Write};

pub struct Serial(pub Box<dyn serialport::SerialPort>);

/// Helper to convert std::io::Error to the nb::Error
fn translate_io_errors(err: std::io::Error) -> nb::Error<hal::serial::ErrorKind> {
    match err.kind() {
        IoErrorKind::WouldBlock | IoErrorKind::TimedOut | IoErrorKind::Interrupted => {
            nb::Error::WouldBlock
        }
        _err => nb::Error::Other(hal::serial::ErrorKind::Other),
    }
}

impl hal::serial::nb::Read<u8> for Serial {
    type Error = hal::serial::ErrorKind;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buffer = [0; 1];
        let bytes_read = self.0.read(&mut buffer).map_err(translate_io_errors)?;
        if bytes_read == 1 {
            Ok(buffer[0])
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl hal::serial::nb::Write<u8> for Serial {
    type Error = hal::serial::ErrorKind;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.0.write(&[word]).map_err(translate_io_errors)?;
        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.0.flush().map_err(translate_io_errors)
    }
}
