use heapless::ArrayLength;

use super::{Error, Result, RingBuffer, Socket, SocketHandle, SocketMeta};

/// A TCP socket ring buffer.
pub type SocketBuffer<N> = RingBuffer<u8, N>;

#[derive(Debug, PartialEq, Eq, Clone, Copy, defmt::Format)]
pub enum State {
    /// Freshly created, unsullied
    Created,
    /// TCP connected or UDP has an address
    Connected,
    /// Block all reads
    ShutdownForRead,
    /// Block all writes
    ShutdownForWrite,
    /// Block all reads and writes, waiting for far end to complete closure, can
    /// be tidied up.
    Closing,
}

impl Default for State {
    fn default() -> Self {
        State::Created
    }
}

/// A Transmission Control Protocol socket.
///
/// A TCP socket may passively listen for connections or actively connect to another endpoint.
/// Note that, for listening sockets, there is no "backlog"; to be able to simultaneously
/// accept several connections, as many sockets must be allocated, or any new connection
/// attempts will be reset.
pub struct TcpSocket<L: ArrayLength<u8>> {
    pub(crate) meta: SocketMeta,
    state: State,
    available_data: usize,
    rx_buffer: SocketBuffer<L>,
}

impl<L: ArrayLength<u8>> TcpSocket<L> {
    /// Create a socket using the given buffers.
    pub fn new(socket_id: u8) -> TcpSocket<L> {
        TcpSocket {
            meta: SocketMeta {
                handle: SocketHandle(socket_id),
            },
            state: State::default(),
            available_data: 0,
            rx_buffer: SocketBuffer::new(),
        }
    }

    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta.handle
    }

    /// Return the connection state, in terms of the TCP state machine.
    #[inline]
    pub fn state(&self) -> State {
        self.state
    }

    /// Set available data.
    pub fn set_available_data(&mut self, available_data: usize) {
        self.available_data = available_data;
    }

    /// Get the number of bytes available to ingress.
    pub fn get_available_data(&self) -> usize {
        self.available_data
    }

    /// Return whether a connection is active.
    ///
    /// This function returns true if the socket is actively exchanging packets
    /// with a remote endpoint. Note that this does not mean that it is possible
    /// to send or receive data through the socket; for that, use
    /// [can_send](#method.can_send) or [can_recv](#method.can_recv).
    ///
    /// If a connection is established, [abort](#method.close) will send a reset
    /// to the remote endpoint.
    ///
    /// In terms of the TCP state machine, the socket must be in the `Closed` or
    /// `ShutdownForRead` state.
    #[inline]
    pub fn is_active(&self) -> bool {
        !matches!(self.state, State::ShutdownForRead)
    }

    /// Return whether the transmit half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to send data and have it arrive
    /// to the remote endpoint. However, it does not make any guarantees about the state
    /// of the transmit buffer, and even if it returns true, [send](#method.send) may
    /// not be able to enqueue any octets.
    ///
    /// In terms of the TCP state machine, the socket must be in the `Connected` or
    /// `ShutdownForRead` state.
    #[inline]
    pub fn may_send(&self) -> bool {
        // In `ShutdownForRead`, the remote endpoint has closed our receive half of the connection
        // but we still can transmit indefinitely.
        matches!(self.state, State::Connected | State::ShutdownForRead)
    }

    /// Return whether the receive half of the full-duplex connection is open.
    ///
    /// This function returns true if it's possible to receive data from the remote endpoint.
    /// It will return true while there is data in the receive buffer, and if there isn't,
    /// as long as the remote endpoint has not closed the connection.
    ///
    /// In terms of the TCP state machine, the socket must be in the `Connected`,
    /// `FIN-WAIT-1`, or `FIN-WAIT-2` state, or have data in the receive buffer instead.
    #[inline]
    pub fn may_recv(&self) -> bool {
        match self.state {
            State::Connected => true,
            State::ShutdownForWrite => true,
            // If we have something in the receive buffer, we can receive that.
            _ if !self.rx_buffer.is_empty() => true,
            _ => false,
        }
    }

    /// Check whether the receive half of the full-duplex connection buffer is open
    /// (see [may_recv](#method.may_recv), and the receive buffer is not full.
    #[inline]
    pub fn can_recv(&self) -> bool {
        if !self.may_recv() {
            return false;
        }

        !self.rx_buffer.is_full()
    }

    fn recv_impl<'b, F, R>(&'b mut self, f: F) -> Result<R>
    where
        F: FnOnce(&'b mut SocketBuffer<L>) -> (usize, R),
    {
        // We may have received some data inside the initial SYN, but until the connection
        // is fully open we must not dequeue any data, as it may be overwritten by e.g.
        // another (stale) SYN. (We do not support TCP Fast Open.)
        if !self.may_recv() {
            return Err(Error::Illegal);
        }

        let (_size, result) = f(&mut self.rx_buffer);
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

    /// Call `f` with a slice of octets in the receive buffer, and dequeue the
    /// amount of elements returned by `f`.
    ///
    /// If the buffer read wraps around, the second argument of `f` will be
    /// `Some()` with the remainder of the buffer, such that the combined slice
    /// of the two arguments, makes up the full buffer.
    ///
    /// This function returns `Err(Error::Illegal) if the receive half of the
    /// connection is not open; see [may_recv](#method.may_recv).
    pub fn recv_wrapping<'b, F>(&'b mut self, f: F) -> Result<usize>
    where
        F: FnOnce(&'b [u8], Option<&'b [u8]>) -> usize,
    {
        self.recv_impl(|rx_buffer| {
            rx_buffer.dequeue_many_with_wrapping(|a, b| {
                let len = f(a, b);
                (len, len)
            })
        })
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

        Ok(self.rx_buffer.get_allocated(0, size))
    }

    pub fn rx_window(&self) -> usize {
        self.rx_buffer.window()
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

impl<L: ArrayLength<u8>> Into<Socket<L>> for TcpSocket<L> {
    fn into(self) -> Socket<L> {
        Socket::Tcp(self)
    }
}
