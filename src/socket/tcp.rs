use core::fmt;

use heapless::consts;

use super::{Error, Result};
use crate::socket::{RingBuffer, Socket, SocketHandle, SocketMeta};

/// A TCP socket ring buffer.
pub type SocketBuffer<N> = RingBuffer<u8, N>;

/// The state of a TCP socket, according to [RFC 793].
///
/// [RFC 793]: https://tools.ietf.org/html/rfc793
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

impl Default for State {
    fn default() -> Self {
        State::Closed
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            State::Closed => write!(f, "CLOSED"),
            State::Listen => write!(f, "LISTEN"),
            State::SynSent => write!(f, "SYN-SENT"),
            State::SynReceived => write!(f, "SYN-RECEIVED"),
            State::Established => write!(f, "ESTABLISHED"),
            State::FinWait1 => write!(f, "FIN-WAIT-1"),
            State::FinWait2 => write!(f, "FIN-WAIT-2"),
            State::CloseWait => write!(f, "CLOSE-WAIT"),
            State::Closing => write!(f, "CLOSING"),
            State::LastAck => write!(f, "LAST-ACK"),
            State::TimeWait => write!(f, "TIME-WAIT"),
        }
    }
}

/// A Transmission Control Protocol socket.
///
/// A TCP socket may passively listen for connections or actively connect to another endpoint.
/// Note that, for listening sockets, there is no "backlog"; to be able to simultaneously
/// accept several connections, as many sockets must be allocated, or any new connection
/// attempts will be reset.
#[derive(Debug)]
pub struct TcpSocket {
    pub(crate) meta: SocketMeta,
    state: State,
    rx_buffer: SocketBuffer<consts::U256>,
    // tx_buffer: SocketBuffer<consts::U512>,
}

impl TcpSocket {
    #[allow(unused_comparisons)] // small usize platforms always pass rx_capacity check
    /// Create a socket using the given buffers.
    pub fn new(socket_id: usize) -> TcpSocket {
        TcpSocket {
            meta: SocketMeta {
                handle: SocketHandle(socket_id),
            },
            state: State::Closed,
            rx_buffer: SocketBuffer::new(),
            // tx_buffer: SocketBuffer::new()
        }
    }

    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta.handle
    }

    //     /// Return the timeout duration.
    //     ///
    //     /// See also the [set_timeout](#method.set_timeout) method.
    //     pub fn timeout(&self) -> Option<Duration> {
    //         self.timeout
    //     }

    //     /// Return the current window field value, including scaling according to RFC 1323.
    //     ///
    //     /// Used in internal calculations as well as packet generation.
    //     ///
    //     #[inline]
    //     fn scaled_window(&self) -> u16 {
    //         cmp::min(self.rx_buffer.window() >> self.remote_win_shift as usize,
    //                  (1 << 16) - 1) as u16
    //     }

    //     /// Set the timeout duration.
    //     ///
    //     /// A socket with a timeout duration set will abort the connection if either of the following
    //     /// occurs:
    //     ///
    //     ///   * After a [connect](#method.connect) call, the remote endpoint does not respond within
    //     ///     the specified duration;
    //     ///   * After establishing a connection, there is data in the transmit buffer and the remote
    //     ///     endpoint exceeds the specified duration between any two packets it sends;
    //     ///   * After enabling [keep-alive](#method.set_keep_alive), the remote endpoint exceeds
    //     ///     the specified duration between any two packets it sends.
    //     pub fn set_timeout(&mut self, duration: Option<Duration>) {
    //         self.timeout = duration
    //     }

    //     /// Return the keep-alive interval.
    //     ///
    //     /// See also the [set_keep_alive](#method.set_keep_alive) method.
    //     pub fn keep_alive(&self) -> Option<Duration> {
    //         self.keep_alive
    //     }

    //     /// Set the keep-alive interval.
    //     ///
    //     /// An idle socket with a keep-alive interval set will transmit a "challenge ACK" packet
    //     /// every time it receives no communication during that interval. As a result, three things
    //     /// may happen:
    //     ///
    //     ///   * The remote endpoint is fine and answers with an ACK packet.
    //     ///   * The remote endpoint has rebooted and answers with an RST packet.
    //     ///   * The remote endpoint has crashed and does not answer.
    //     ///
    //     /// The keep-alive functionality together with the timeout functionality allows to react
    //     /// to these error conditions.
    //     pub fn set_keep_alive(&mut self, interval: Option<Duration>) {
    //         self.keep_alive = interval;
    //         if self.keep_alive.is_some() {
    //             // If the connection is idle and we've just set the option, it would not take effect
    //             // until the next packet, unless we wind up the timer explicitly.
    //             self.timer.set_keep_alive();
    //         }
    //     }

    //     /// Return the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    //     ///
    //     /// See also the [set_hop_limit](#method.set_hop_limit) method
    //     pub fn hop_limit(&self) -> Option<u8> {
    //         self.hop_limit
    //     }

    //     /// Set the time-to-live (IPv4) or hop limit (IPv6) value used in outgoing packets.
    //     ///
    //     /// A socket without an explicitly set hop limit value uses the default [IANA recommended]
    //     /// value (64).
    //     ///
    //     /// # Panics
    //     ///
    //     /// This function panics if a hop limit value of 0 is given. See [RFC 1122 § 3.2.1.7].
    //     ///
    //     /// [IANA recommended]: https://www.iana.org/assignments/ip-parameters/ip-parameters.xhtml
    //     /// [RFC 1122 § 3.2.1.7]: https://tools.ietf.org/html/rfc1122#section-3.2.1.7
    //     pub fn set_hop_limit(&mut self, hop_limit: Option<u8>) {
    //         // A host MUST NOT send a datagram with a hop limit value of 0
    //         if let Some(0) = hop_limit {
    //             panic!("the time-to-live value of a packet must not be zero")
    //         }

    //         self.hop_limit = hop_limit
    //     }

    //     /// Return the local endpoint.
    //     #[inline]
    //     pub fn local_endpoint(&self) -> IpEndpoint {
    //         self.local_endpoint
    //     }

    //     /// Return the remote endpoint.
    //     #[inline]
    //     pub fn remote_endpoint(&self) -> IpEndpoint {
    //         self.remote_endpoint
    //     }

    /// Return the connection state, in terms of the TCP state machine.
    #[inline]
    pub fn state(&self) -> State {
        self.state
    }

    //     fn reset(&mut self) {
    //         let rx_cap_log2 = mem::size_of::<usize>() * 8 -
    //             self.rx_buffer.capacity().leading_zeros() as usize;

    //         self.state           = State::Closed;
    //         self.timer           = Timer::default();
    //         self.assembler       = Assembler::new(self.rx_buffer.capacity());
    //         self.tx_buffer.clear();
    //         self.rx_buffer.clear();
    //         self.keep_alive      = None;
    //         self.timeout         = None;
    //         self.hop_limit       = None;
    //         self.listen_address  = IpAddress::default();
    //         self.local_endpoint  = IpEndpoint::default();
    //         self.remote_endpoint = IpEndpoint::default();
    //         self.local_seq_no    = TcpSeqNumber::default();
    //         self.remote_seq_no   = TcpSeqNumber::default();
    //         self.remote_last_seq = TcpSeqNumber::default();
    //         self.remote_last_ack = None;
    //         self.remote_last_win = 0;
    //         self.remote_win_len  = 0;
    //         self.remote_win_scale = None;
    //         self.remote_win_shift = rx_cap_log2.saturating_sub(16) as u8;
    //         self.remote_mss      = DEFAULT_MSS;
    //         self.remote_last_ts  = None;
    //     }

    //     /// Start listening on the given endpoint.
    //     ///
    //     /// This function returns `Err(Error::Illegal)` if the socket was already open
    //     /// (see [is_open](#method.is_open)), and `Err(Error::Unaddressable)`
    //     /// if the port in the given endpoint is zero.
    //     pub fn listen<T>(&mut self, local_endpoint: T) -> Result<()>
    //             where T: Into<IpEndpoint> {
    //         let local_endpoint = local_endpoint.into();
    //         if local_endpoint.port == 0 { return Err(Error::Unaddressable) }

    //         if self.is_open() { return Err(Error::Illegal) }

    //         self.reset();
    //         self.listen_address  = local_endpoint.addr;
    //         self.local_endpoint  = local_endpoint;
    //         self.remote_endpoint = IpEndpoint::default();
    //         self.set_state(State::Listen);
    //         Ok(())
    //     }

    //     /// Connect to a given endpoint.
    //     ///
    //     /// The local port must be provided explicitly. Assuming `fn get_ephemeral_port() -> u16`
    //     /// allocates a port between 49152 and 65535, a connection may be established as follows:
    //     ///
    //     /// ```rust,ignore
    //     /// socket.connect((IpAddress::v4(10, 0, 0, 1), 80), get_ephemeral_port())
    //     /// ```
    //     ///
    //     /// The local address may optionally be provided.
    //     ///
    //     /// This function returns an error if the socket was open; see [is_open](#method.is_open).
    //     /// It also returns an error if the local or remote port is zero, or if the remote address
    //     /// is unspecified.
    //     pub fn connect<T, U>(&mut self, remote_endpoint: T, local_endpoint: U) -> Result<()>
    //             where T: Into<IpEndpoint>, U: Into<IpEndpoint> {
    //         let remote_endpoint = remote_endpoint.into();
    //         let local_endpoint  = local_endpoint.into();

    //         if self.is_open() { return Err(Error::Illegal) }
    //         if !remote_endpoint.is_specified() { return Err(Error::Unaddressable) }
    //         if local_endpoint.port == 0 { return Err(Error::Unaddressable) }

    //         // If local address is not provided, use an unspecified address but a specified protocol.
    //         // This lets us lower IpRepr later to determine IP header size and calculate MSS,
    //         // but without committing to a specific address right away.
    //         let local_addr = match remote_endpoint.addr {
    //             IpAddress::Unspecified => return Err(Error::Unaddressable),
    //             _ => remote_endpoint.addr.to_unspecified(),
    //         };
    //         let local_endpoint = IpEndpoint { addr: local_addr, ..local_endpoint };

    //         // Carry over the local sequence number.
    //         let local_seq_no = self.local_seq_no;

    //         self.reset();
    //         self.local_endpoint  = local_endpoint;
    //         self.remote_endpoint = remote_endpoint;
    //         self.local_seq_no    = local_seq_no;
    //         self.remote_last_seq = local_seq_no;
    //         self.set_state(State::SynSent);
    //         Ok(())
    //     }

    /// Close the connection.
    pub fn close(&mut self) {
        self.set_state(State::Closed);
    }

    /// Return whether the socket is passively listening for incoming connections.
    ///
    /// In terms of the TCP state machine, the socket must be in the `LISTEN` state.
    #[inline]
    pub fn is_listening(&self) -> bool {
        match self.state {
            State::Listen => true,
            _ => false,
        }
    }

    /// Return whether the socket is open.
    ///
    /// This function returns true if the socket will process incoming or dispatch outgoing
    /// packets. Note that this does not mean that it is possible to send or receive data through
    /// the socket; for that, use [can_send](#method.can_send) or [can_recv](#method.can_recv).
    ///
    /// In terms of the TCP state machine, the socket must not be in the `CLOSED`
    /// or `TIME-WAIT` states.
    #[inline]
    pub fn is_open(&self) -> bool {
        match self.state {
            State::Closed => false,
            State::TimeWait => false,
            _ => true,
        }
    }

    /// Return whether a connection is active.
    ///
    /// This function returns true if the socket is actively exchanging packets with
    /// a remote endpoint. Note that this does not mean that it is possible to send or receive
    /// data through the socket; for that, use [can_send](#method.can_send) or
    /// [can_recv](#method.can_recv).
    ///
    /// If a connection is established, [abort](#method.close) will send a reset to
    /// the remote endpoint.
    ///
    /// In terms of the TCP state machine, the socket must be in the `CLOSED`, `TIME-WAIT`,
    /// or `LISTEN` state.
    #[inline]
    pub fn is_active(&self) -> bool {
        match self.state {
            State::Closed => false,
            State::TimeWait => false,
            State::Listen => false,
            _ => true,
        }
    }

    /// Return whether the transmit half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to send data and have it arrive
    /// to the remote endpoint. However, it does not make any guarantees about the state
    /// of the transmit buffer, and even if it returns true, [send](#method.send) may
    /// not be able to enqueue any octets.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED` or
    /// `CLOSE-WAIT` state.
    #[inline]
    pub fn may_send(&self) -> bool {
        match self.state {
            State::Established => true,
            // In CLOSE-WAIT, the remote endpoint has closed our receive half of the connection
            // but we still can transmit indefinitely.
            State::CloseWait => true,
            _ => false,
        }
    }

    /// Return whether the receive half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to receive data from the remote endpoint.
    /// It will return true while there is data in the receive buffer, and if there isn't,
    /// as long as the remote endpoint has not closed the connection.
    ///
    /// In terms of the TCP state machine, the socket must be in the `ESTABLISHED`,
    /// `FIN-WAIT-1`, or `FIN-WAIT-2` state, or have data in the receive buffer instead.
    #[inline]
    pub fn may_recv(&self) -> bool {
        match self.state {
            State::Established => true,
            // In FIN-WAIT-1/2, we have closed our transmit half of the connection but
            // we still can receive indefinitely.
            State::FinWait1 | State::FinWait2 => true,
            // If we have something in the receive buffer, we can receive that.
            _ if self.rx_buffer.len() > 0 => true,
            _ => false,
        }
    }

    //     /// Check whether the transmit half of the full-duplex connection is open
    //     /// (see [may_send](#method.may_send), and the transmit buffer is not full.
    //     #[inline]
    //     pub fn can_send(&self) -> bool {
    //         if !self.may_send() { return false }

    //         !self.tx_buffer.is_full()
    //     }

    /// Check whether the receive half of the full-duplex connection buffer is open
    /// (see [may_recv](#method.may_recv), and the receive buffer is not empty.
    #[inline]
    pub fn can_recv(&self) -> bool {
        if !self.may_recv() {
            return false;
        }

        !self.rx_buffer.is_empty()
    }

    //     fn send_impl<'b, F, R>(&'b mut self, f: F) -> Result<R>
    //             where F: FnOnce(&'b mut SocketBuffer<'a>) -> (usize, R) {
    //         if !self.may_send() { return Err(Error::Illegal) }

    //         // The connection might have been idle for a long time, and so remote_last_ts
    //         // would be far in the past. Unless we clear it here, we'll abort the connection
    //         // down over in dispatch() by erroneously detecting it as timed out.
    //         if self.tx_buffer.is_empty() { self.remote_last_ts = None }

    //         let _old_length = self.tx_buffer.len();
    //         let (size, result) = f(&mut self.tx_buffer);
    //         if size > 0 {
    //             #[cfg(any(test, feature = "verbose"))]
    //             net_trace!("{}:{}:{}: tx buffer: enqueueing {} octets (now {})",
    //                        self.meta.handle, self.local_endpoint, self.remote_endpoint,
    //                        size, _old_length + size);
    //         }
    //         Ok(result)
    //     }

    //     /// Call `f` with the largest contiguous slice of octets in the transmit buffer,
    //     /// and enqueue the amount of elements returned by `f`.
    //     ///
    //     /// This function returns `Err(Error::Illegal) if the transmit half of
    //     /// the connection is not open; see [may_send](#method.may_send).
    //     pub fn send<'b, F, R>(&'b mut self, f: F) -> Result<R>
    //             where F: FnOnce(&'b mut [u8]) -> (usize, R) {
    //         self.send_impl(|tx_buffer| {
    //             tx_buffer.enqueue_many_with(f)
    //         })
    //     }

    //     /// Enqueue a sequence of octets to be sent, and fill it from a slice.
    //     ///
    //     /// This function returns the amount of bytes actually enqueued, which is limited
    //     /// by the amount of free space in the transmit buffer; down to zero.
    //     ///
    //     /// See also [send](#method.send).
    //     pub fn send_slice(&mut self, data: &[u8]) -> Result<usize> {
    //         self.send_impl(|tx_buffer| {
    //             let size = tx_buffer.enqueue_slice(data);
    //             (size, size)
    //         })
    //     }

    fn recv_impl<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut SocketBuffer<consts::U256>) -> (usize, R),
    {
        // We may have received some data inside the initial SYN, but until the connection
        // is fully open we must not dequeue any data, as it may be overwritten by e.g.
        // another (stale) SYN. (We do not support TCP Fast Open.)
        if !self.may_recv() {
            return Err(Error::Illegal);
        }

        let (size, result) = f(&mut self.rx_buffer);
        Ok(result)
    }

    /// Call `f` with the largest contiguous slice of octets in the receive buffer,
    /// and dequeue the amount of elements returned by `f`.
    ///
    /// This function returns `Err(Error::Illegal) if the receive half of
    /// the connection is not open; see [may_recv](#method.may_recv).
    pub fn recv<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut [u8]) -> (usize, R),
    {
        self.recv_impl(|rx_buffer| rx_buffer.dequeue_many_with(f))
    }

    /// Dequeue a sequence of received octets, and fill a slice from it.
    ///
    /// This function returns the amount of bytes actually dequeued, which is limited
    /// by the amount of free space in the transmit buffer; down to zero.
    ///
    /// See also [recv](#method.recv).
    pub fn recv_slice(&mut self, data: &mut [u8]) -> Result<usize> {

        self.recv_impl(|rx_buffer| {
            let size = rx_buffer.dequeue_slice(data);
            (size, size)
        })
    }

    /// Peek at a sequence of received octets without removing them from
    /// the receive buffer, and return a pointer to it.
    ///
    /// This function otherwise behaves identically to [recv](#method.recv).
    pub fn peek(&mut self, size: usize) -> Result<&[u8]> {
        // See recv() above.
        if !self.may_recv() {
            return Err(Error::Illegal);
        }

        let buffer = self.rx_buffer.get_allocated(0, size);
        // if buffer.len() > 0 {
        //     #[cfg(any(test, feature = "verbose"))]
        //     net_trace!("{}:{}:{}: rx buffer: peeking at {} octets",
        //                self.meta.handle, self.local_endpoint, self.remote_endpoint,
        //                buffer.len());
        // }
        Ok(buffer)
    }

    /// Peek at a sequence of received octets without removing them from
    /// the receive buffer, and fill a slice from it.
    ///
    /// This function otherwise behaves identically to [recv_slice](#method.recv_slice).
    pub fn peek_slice(&mut self, data: &mut [u8]) -> Result<usize> {
        let buffer = self.peek(data.len())?;
        let data = &mut data[..buffer.len()];
        data.copy_from_slice(buffer);
        Ok(buffer.len())
    }

    //     /// Return the amount of octets queued in the transmit buffer.
    //     ///
    //     /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    //     pub fn send_queue(&self) -> usize {
    //         self.tx_buffer.len()
    //     }

    pub fn rx_enqueue_slice(&mut self, data: &[u8]) -> usize {
        self.rx_buffer.enqueue_slice(data)
    }
    /// Return the amount of octets queued in the receive buffer.
    ///
    /// Note that the Berkeley sockets interface does not have an equivalent of this API.
    pub fn recv_queue(&self) -> usize {
        self.rx_buffer.len()
    }

    pub fn set_state(&mut self, state: State) {
        // if self.state != state {
        //     if self.remote_endpoint.addr.is_unspecified() {
        //         net_trace!("{}:{}: state={}=>{}",
        //                    self.meta.handle, self.local_endpoint,
        //                    self.state, state);
        //     } else {
        //         net_trace!("{}:{}:{}: state={}=>{}",
        //                    self.meta.handle, self.local_endpoint, self.remote_endpoint,
        //                    self.state, state);
        //     }
        // }
        self.state = state
    }
}

impl Into<Socket> for TcpSocket {
    fn into(self) -> Socket {
        Socket::Tcp(self)
    }
}

// impl fmt::Write for TcpSocket {
//     fn write_str(&mut self, slice: &str) -> fmt::Result {
//         let slice = slice.as_bytes();
//         if self.send_slice(slice) == Ok(slice.len()) {
//             Ok(())
//         } else {
//             Err(fmt::Error)
//         }
//     }
// }
