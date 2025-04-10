use core::result::Result;
use embassy_rp::{peripherals::UART0, uart::BufferedUart};
pub struct CellTransport(pub BufferedUart<'static, UART0>);

impl ublox_cellular::config::Transport for CellTransport {
    fn set_baudrate(&mut self, baudrate: u32) {
        self.0.set_baudrate(baudrate)
    }

    fn split_ref(
        &mut self,
    ) -> (
        impl embedded_io_async::Write,
        impl embedded_io_async::Read + embedded_io_async::BufRead,
    ) {
        self.0.split_ref()
    }
}
impl embedded_io_async::ErrorType for CellTransport {
    type Error = embassy_rp::uart::Error;
}

impl embedded_io_async::Read for CellTransport {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.0.read(buf).await
    }
}

impl embedded_io_async::BufRead for CellTransport {
    async fn fill_buf(&mut self) -> Result<&[u8], Self::Error> {
        self.0.fill_buf().await
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}

impl embedded_io_async::Write for CellTransport {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await
    }
}
