use core::cmp::min;
use core::convert::TryInto;

use embedded_time::{duration::*, Clock, Instant};

use super::{Error, Result, RingBuffer, Socket, SocketHandle, SocketMeta};
pub use embedded_nal::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// A UDP socket ring buffer.
pub type SocketBuffer<const N: usize> = RingBuffer<u8, N>;

/// A User Datagram Protocol socket.
///
/// A UDP socket is bound to a specific endpoint, and owns transmit and receive
/// packet buffers.
pub struct UdpSocket<CLK: Clock, const L: usize> {
    pub(crate) meta: SocketMeta,
    pub(crate) endpoint: SocketAddr,
    check_interval: Seconds<u32>,
    read_timeout: Option<Seconds<u32>>,
    available_data: usize,
    rx_buffer: SocketBuffer<L>,
    last_check_time: Option<Instant<CLK>>,
    closed_time: Option<Instant<CLK>>,
}

impl<CLK: Clock, const L: usize> UdpSocket<CLK, L> {
    /// Create an UDP socket with the given buffers.
    pub fn new(socket_id: u8) -> UdpSocket<CLK, L> {
        UdpSocket {
            meta: SocketMeta {
                handle: SocketHandle(socket_id),
            },
            check_interval: Seconds(15),
            read_timeout: Some(Seconds(15)),
            endpoint: SocketAddrV4::new(Ipv4Addr::unspecified(), 0).into(),
            available_data: 0,
            rx_buffer: SocketBuffer::new(),
            last_check_time: None,
            closed_time: None,
        }
    }

    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta.handle
    }

    /// Return the bound endpoint.
    #[inline]
    pub fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }

    pub fn should_update_available_data(&mut self, ts: Instant<CLK>) -> bool
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        self.last_check_time
            .replace(ts)
            .and_then(|ref last_check_time| ts.checked_duration_since(last_check_time))
            .and_then(|dur| dur.try_into().ok())
            .map(|dur: Milliseconds<u32>| dur >= self.check_interval)
            .unwrap_or(false)
    }

    pub fn recycle(&self, ts: &Instant<CLK>) -> bool
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        if let Some(read_timeout) = self.read_timeout {
            self.closed_time
                .and_then(|ref closed_time| ts.checked_duration_since(closed_time))
                .and_then(|dur| dur.try_into().ok())
                .map(|dur: Milliseconds<u32>| dur >= read_timeout)
                .unwrap_or(false)
        } else {
            false
        }
    }

    pub fn closed_by_remote(&mut self, ts: Instant<CLK>)
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        self.closed_time.replace(ts);
    }

    /// Set available data.
    pub fn set_available_data(&mut self, available_data: usize) {
        self.available_data = available_data;
    }

    /// Get the number of bytes available to ingress.
    pub fn get_available_data(&self) -> usize {
        self.available_data
    }

    pub fn rx_window(&self) -> usize {
        self.rx_buffer.window()
    }

    /// Bind the socket to the given endpoint.
    ///
    /// This function returns `Err(Error::Illegal)` if the socket was open
    /// (see [is_open](#method.is_open)), and `Err(Error::Unaddressable)`
    /// if the port in the given endpoint is zero.
    pub fn bind<T: Into<SocketAddr>>(&mut self, endpoint: T) -> Result<()> {
        if self.is_open() {
            return Err(Error::Illegal);
        }

        let endpoint = endpoint.into();
        match endpoint {
            SocketAddr::V4(ipv4) => {
                if ipv4.port() == 0 {
                    return Err(Error::Unaddressable);
                }
            }
            SocketAddr::V6(ipv6) => {
                if ipv6.port() == 0 {
                    return Err(Error::Unaddressable);
                }
            }
        }

        self.endpoint = endpoint;
        Ok(())
    }

    /// Check whether the socket is open.
    #[inline]
    pub fn is_open(&self) -> bool {
        match self.endpoint {
            SocketAddr::V4(ipv4) => ipv4.port() != 0,
            SocketAddr::V6(ipv6) => ipv6.port() != 0,
        }
    }

    /// Check whether the receive buffer is full.
    #[inline]
    pub fn can_recv(&self) -> bool {
        !self.rx_buffer.is_full()
    }

    // /// Return the maximum number packets the socket can receive.
    // #[inline]
    // pub fn packet_recv_capacity(&self) -> usize {
    //     self.rx_buffer.packet_capacity()
    // }

    // /// Return the maximum number of bytes inside the recv buffer.
    // #[inline]
    // pub fn payload_recv_capacity(&self) -> usize {
    //     self.rx_buffer.payload_capacity()
    // }

    fn recv_impl<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut SocketBuffer<L>) -> (usize, R),
    {
        // We may have received some data inside the initial SYN, but until the connection
        // is fully open we must not dequeue any data, as it may be overwritten by e.g.
        // another (stale) SYN. (We do not support TCP Fast Open.)
        if !self.is_open() {
            return Err(Error::Illegal);
        }

        let (_size, result) = f(&mut self.rx_buffer);
        Ok(result)
    }

    /// Dequeue a packet received from a remote endpoint, and return the endpoint as well
    /// as a pointer to the payload.
    ///
    /// This function returns `Err(Error::Exhausted)` if the receive buffer is empty.
    pub fn recv<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut [u8]) -> (usize, R),
    {
        self.recv_impl(|rx_buffer| rx_buffer.dequeue_many_with(f))
    }

    /// Dequeue a packet received from a remote endpoint, copy the payload into the given slice,
    /// and return the amount of octets copied as well as the endpoint.
    ///
    /// See also [recv](#method.recv).
    pub fn recv_slice(&mut self, data: &mut [u8]) -> Result<usize> {
        self.recv_impl(|rx_buffer| {
            let size = rx_buffer.dequeue_slice(data);
            (size, size)
        })
    }

    pub fn rx_enqueue_slice(&mut self, data: &[u8]) -> usize {
        self.rx_buffer.enqueue_slice(data)
    }

    /// Peek at a packet received from a remote endpoint, and return the endpoint as well
    /// as a pointer to the payload without removing the packet from the receive buffer.
    /// This function otherwise behaves identically to [recv](#method.recv).
    ///
    /// It returns `Err(Error::Exhausted)` if the receive buffer is empty.
    pub fn peek(&mut self, size: usize) -> Result<&[u8]> {
        if !self.is_open() {
            return Err(Error::Illegal);
        }

        Ok(self.rx_buffer.get_allocated(0, size))
    }

    /// Peek at a packet received from a remote endpoint, copy the payload into the given slice,
    /// and return the amount of octets copied as well as the endpoint without removing the
    /// packet from the receive buffer.
    /// This function otherwise behaves identically to [recv_slice](#method.recv_slice).
    ///
    /// See also [peek](#method.peek).
    pub fn peek_slice(&mut self, data: &mut [u8]) -> Result<usize> {
        let buffer = self.peek(data.len())?;
        let length = min(data.len(), buffer.len());
        data[..length].copy_from_slice(&buffer[..length]);
        Ok(length)
    }

    pub fn close(&mut self) {
        self.endpoint.set_port(0);
    }
}

impl<CLK: Clock, const L: usize> Into<Socket<CLK, L>> for UdpSocket<CLK, L> {
    fn into(self) -> Socket<CLK, L> {
        Socket::Udp(self)
    }
}
