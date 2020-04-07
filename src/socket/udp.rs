use core::cmp::min;

use core::fmt;

use heapless::consts;

use super::{Error, Result};
use crate::socket::{RingBuffer, Socket, SocketHandle, SocketMeta};
pub use embedded_nal::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// A UDP socket ring buffer.
pub type SocketBuffer<N> = RingBuffer<u8, N>;

/// The state of a TCP socket, according to [RFC 793].
///
/// [RFC 793]: https://tools.ietf.org/html/rfc793
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Closed,
    Open,
}

impl Default for State {
    fn default() -> Self {
        State::Open
    }
}

/// A User Datagram Protocol socket.
///
/// A UDP socket is bound to a specific endpoint, and owns transmit and receive
/// packet buffers.
#[derive(Debug)]
pub struct UdpSocket {
    pub(crate) meta: SocketMeta,
    pub(crate) endpoint: SocketAddr,
    rx_buffer: SocketBuffer<consts::U256>,
    state: State,
    /// The time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    hop_limit: Option<u8>,
}

impl UdpSocket {
    /// Create an UDP socket with the given buffers.
    pub fn new(socket_id: usize) -> UdpSocket {
        UdpSocket {
            meta: SocketMeta {
                handle: SocketHandle(socket_id),
            },
            endpoint: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 0)),
            rx_buffer: SocketBuffer::new(),
            state: State::default(),
            hop_limit: None,
        }
    }

    // /// Return the socket handle.
    // #[inline]
    // pub fn handle(&self) -> SocketHandle {
    //     self.meta.handle
    // }

    // /// Return the bound endpoint.
    // #[inline]
    // pub fn endpoint(&self) -> IpEndpoint {
    //     self.endpoint
    // }

    /// Return the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    ///
    /// See also the [set_hop_limit](#method.set_hop_limit) method
    pub fn hop_limit(&self) -> Option<u8> {
        self.hop_limit
    }

    /// Set the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    ///
    /// A socket without an explicitly set hop limit value uses the default [IANA recommended]
    /// value (64).
    ///
    /// # Panics
    ///
    /// This function panics if a hop limit value of 0 is given. See [RFC 1122 § 3.2.1.7].
    ///
    /// [IANA recommended]: https://www.iana.org/assignments/ip-parameters/ip-parameters.xhtml
    /// [RFC 1122 § 3.2.1.7]: https://tools.ietf.org/html/rfc1122#section-3.2.1.7
    pub fn set_hop_limit(&mut self, hop_limit: Option<u8>) {
        // A host MUST NOT send a datagram with a hop limit value of 0
        if let Some(0) = hop_limit {
            panic!("the time-to-live value of a packet must not be zero")
        }

        self.hop_limit = hop_limit
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
            SocketAddr::V4(ipv4) => {
                if ipv4.port() == 0 {
                    return false;
                }
            }
            SocketAddr::V6(ipv6) => {
                if ipv6.port() == 0 {
                    return false;
                }
            }
        }
        true
    }

    // /// Check whether the transmit buffer is full.
    // #[inline]
    // pub fn can_send(&self) -> bool {
    //     !self.tx_buffer.is_full()
    // }

    /// Check whether the receive buffer is not empty.
    #[inline]
    pub fn can_recv(&self) -> bool {
        !self.rx_buffer.is_empty()
    }

    // /// Return the maximum number packets the socket can receive.
    // #[inline]
    // pub fn packet_recv_capacity(&self) -> usize {
    //     self.rx_buffer.packet_capacity()
    // }

    // /// Return the maximum number packets the socket can transmit.
    // #[inline]
    // pub fn packet_send_capacity(&self) -> usize {
    //     self.tx_buffer.packet_capacity()
    // }

    // /// Return the maximum number of bytes inside the recv buffer.
    // #[inline]
    // pub fn payload_recv_capacity(&self) -> usize {
    //     self.rx_buffer.payload_capacity()
    // }

    // /// Return the maximum number of bytes inside the transmit buffer.
    // #[inline]
    // pub fn payload_send_capacity(&self) -> usize {
    //     self.tx_buffer.payload_capacity()
    // }

    // /// Enqueue a packet to be sent to a given remote endpoint, and return a pointer
    // /// to its payload.
    // ///
    // /// This function returns `Err(Error::Exhausted)` if the transmit buffer is full,
    // /// `Err(Error::Unaddressable)` if local or remote port, or remote address are unspecified,
    // /// and `Err(Error::Truncated)` if there is not enough transmit buffer capacity
    // /// to ever send this packet.
    // pub fn send(&mut self, size: usize, endpoint: IpEndpoint) -> Result<&mut [u8]> {
    //     if self.endpoint.port == 0 { return Err(Error::Unaddressable) }
    //     if !endpoint.is_specified() { return Err(Error::Unaddressable) }

    //     let payload_buf = self.tx_buffer.enqueue(size, endpoint)?;

    //     net_trace!("{}:{}:{}: buffer to send {} octets",
    //                self.meta.handle, self.endpoint, endpoint, size);
    //     Ok(payload_buf)
    // }

    // /// Enqueue a packet to be sent to a given remote endpoint, and fill it from a slice.
    // ///
    // /// See also [send](#method.send).
    // pub fn send_slice(&mut self, data: &[u8], endpoint: IpEndpoint) -> Result<()> {
    //     self.send(data.len(), endpoint)?.copy_from_slice(data);
    //     Ok(())
    // }

    /// Dequeue a packet received from a remote endpoint, and return the endpoint as well
    /// as a pointer to the payload.
    ///
    /// This function returns `Err(Error::Exhausted)` if the receive buffer is empty.
    // pub fn recv(&mut self) -> Result<&[u8]> {
    //     let (endpoint, payload_buf) = self.rx_buffer.dequeue()?;

    //     // net_trace!("{}:{}:{}: receive {} buffered octets",
    //     //            self.meta.handle, self.endpoint,
    //     //            endpoint, payload_buf.len());
    //     Ok(payload_buf)
    // }

    /// Dequeue a packet received from a remote endpoint, copy the payload into the given slice,
    /// and return the amount of octets copied as well as the endpoint.
    ///
    /// See also [recv](#method.recv).
    // pub fn recv_slice(&mut self, data: &mut [u8]) -> Result<(usize)> {
    //     let (buffer) = self.recv()?;
    //     let length = min(data.len(), buffer.len());
    //     data[..length].copy_from_slice(&buffer[..length]);
    //     Ok(length)
    // }

    fn recv_impl<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut SocketBuffer<consts::U256>) -> (usize, R),
    {
        // We may have received some data inside the initial SYN, but until the connection
        // is fully open we must not dequeue any data, as it may be overwritten by e.g.
        // another (stale) SYN. (We do not support TCP Fast Open.)
        if !self.is_open() {
            return Err(Error::Illegal);
        }

        let (size, result) = f(&mut self.rx_buffer);
        Ok(result)
    }

    pub fn recv<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut [u8]) -> (usize, R),
    {
        self.recv_impl(|rx_buffer| rx_buffer.dequeue_many_with(f))
    }

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

        let buffer = self.rx_buffer.get_allocated(0, size);
        Ok(buffer)

        // let handle = self.meta.handle;
        // self.rx_buffer.peek()
        //     .map(|payload_buf| {
        // net_trace!("{}: peek {} buffered octets",
        //            handle,
        //            payload_buf.len());
        //    (payload_buf)
        // })
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

    // pub(crate) fn accepts(&self, ip_repr: &IpRepr, repr: &UdpRepr) -> bool {
    //     if self.endpoint.port != repr.dst_port { return false }
    //     if !self.endpoint.addr.is_unspecified() &&
    //         self.endpoint.addr != ip_repr.dst_addr() &&
    //         !ip_repr.dst_addr().is_broadcast() &&
    //         !ip_repr.dst_addr().is_multicast() { return false }

    //     true
    // }

    // pub(crate) fn process(&mut self, ip_repr: &IpRepr, repr: &UdpRepr) -> Result<()> {
    //     debug_assert!(self.accepts(ip_repr, repr));

    //     let size = repr.payload.len();

    //     let endpoint = IpEndpoint { addr: ip_repr.src_addr(), port: repr.src_port };
    //     self.rx_buffer.enqueue(size, endpoint)?.copy_from_slice(repr.payload);

    //     // net_trace!("{}:{}:{}: receiving {} octets",
    //     //            self.meta.handle, self.endpoint,
    //     //            endpoint, size);
    //     Ok(())
    // }

    // pub(crate) fn dispatch<F>(&mut self, emit: F) -> Result<()>
    //         where F: FnOnce((IpRepr, UdpRepr)) -> Result<()> {
    //     let handle    = self.handle();
    //     let endpoint  = self.endpoint;
    //     let hop_limit = self.hop_limit.unwrap_or(64);

    //     self.tx_buffer.dequeue_with(|remote_endpoint, payload_buf| {
    //         net_trace!("{}:{}:{}: sending {} octets",
    //                     handle, endpoint,
    //                     endpoint, payload_buf.len());

    //         let repr = UdpRepr {
    //             src_port: endpoint.port,
    //             dst_port: remote_endpoint.port,
    //             payload:  payload_buf,
    //         };
    //         let ip_repr = IpRepr::Unspecified {
    //             src_addr:    endpoint.addr,
    //             dst_addr:    remote_endpoint.addr,
    //             protocol:    IpProtocol::Udp,
    //             payload_len: repr.buffer_len(),
    //             hop_limit:   hop_limit,
    //         };
    //         emit((ip_repr, repr))
    //     })
    // }

    // pub(crate) fn poll_at(&self) -> PollAt {
    //     if self.tx_buffer.is_empty() {
    //         PollAt::Ingress
    //     } else {
    //         PollAt::Now
    //     }
    // }

    pub fn close(&mut self) -> Result<()> {
        match self.endpoint {
            SocketAddr::V4(mut ipv4) => ipv4.set_port(0),
            SocketAddr::V6(mut ipv6) => ipv6.set_port(0),
        }
        Ok(())
    }
}

impl Into<Socket> for UdpSocket {
    fn into(self) -> Socket {
        Socket::Udp(self)
    }
}
