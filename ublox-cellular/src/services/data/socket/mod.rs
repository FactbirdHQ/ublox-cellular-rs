mod meta;
mod ref_;
mod ring_buffer;
mod set;
pub mod tcp;
pub mod udp;

use core::convert::TryInto;

pub(crate) use self::meta::Meta as SocketMeta;
pub use self::ring_buffer::RingBuffer;
use embedded_time::{
    duration::{Generic, Milliseconds},
    Clock, Instant,
};
use heapless::ArrayLength;

#[cfg(feature = "socket-tcp")]
pub use tcp::{State as TcpState, TcpSocket};

#[cfg(feature = "socket-udp")]
pub use udp::UdpSocket;

pub use self::set::{Handle as SocketHandle, Set as SocketSet};

pub use self::ref_::Ref as SocketRef;

/// The error type for the networking stack.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Error {
    /// An operation cannot proceed because a buffer is empty or full.
    Exhausted,
    /// An operation is not permitted in the current state.
    Illegal,
    /// An endpoint or address of a remote host could not be translated to a lower level address.
    /// E.g. there was no an Ethernet address corresponding to an IPv4 address in the ARP cache,
    /// or a TCP connection attempt was made to an unspecified endpoint.
    Unaddressable,

    SocketSetFull,
    InvalidSocket,
    DuplicateSocket,
}

type Result<T> = core::result::Result<T, Error>;

/// A network socket.
///
/// This enumeration abstracts the various types of sockets based on the IP protocol.
/// To downcast a `Socket` value to a concrete socket, use the [AnySocket] trait,
/// e.g. to get `UdpSocket`, call `UdpSocket::downcast(socket)`.
///
/// It is usually more convenient to use [SocketSet::get] instead.
///
/// [AnySocket]: trait.AnySocket.html
/// [SocketSet::get]: struct.SocketSet.html#method.get
#[non_exhaustive]
pub enum Socket<L: ArrayLength<u8>, CLK: Clock> {
    // #[cfg(feature = "socket-raw")]
    // Raw(RawSocket<'a, 'b>),
    // #[cfg(all(
    //     feature = "socket-icmp",
    //     any(feature = "proto-ipv4", feature = "proto-ipv6")
    // ))]
    // Icmp(IcmpSocket<'a, 'b>),
    #[cfg(feature = "socket-udp")]
    Udp(UdpSocket<L, CLK>),
    #[cfg(feature = "socket-tcp")]
    Tcp(TcpSocket<L, CLK>),
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SocketType {
    Udp,
    Tcp,
}

impl<L: ArrayLength<u8>, CLK: Clock> Socket<L, CLK> {
    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta().handle
    }

    pub(crate) fn meta(&self) -> &SocketMeta {
        match self {
            // #[cfg(feature = "socket-raw")]
            // Socket::Raw(ref $( $mut_ )* $socket) => $code,
            // #[cfg(all(feature = "socket-icmp", any(feature = "proto-ipv4", feature = "proto-ipv6")))]
            // Socket::Icmp(ref $( $mut_ )* $socket) => $code,
            #[cfg(feature = "socket-udp")]
            Socket::Udp(ref socket) => &socket.meta,
            #[cfg(feature = "socket-tcp")]
            Socket::Tcp(ref socket) => &socket.meta,
        }
    }

    pub fn get_type(&self) -> SocketType {
        match self {
            Socket::Tcp(_) => SocketType::Tcp,
            Socket::Udp(_) => SocketType::Udp,
        }
    }

    pub fn should_update_available_data(&mut self, ts: Instant<CLK>) -> bool
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        match self {
            Socket::Tcp(s) => s.should_update_available_data(ts),
            Socket::Udp(s) => s.should_update_available_data(ts),
        }
    }

    pub fn available_data(&self) -> usize {
        match self {
            Socket::Tcp(s) => s.get_available_data(),
            Socket::Udp(s) => s.get_available_data(),
        }
    }

    pub fn recycle(&self, ts: &Instant<CLK>) -> bool
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        match self {
            Socket::Tcp(s) => s.recycle(ts),
            Socket::Udp(s) => s.recycle(ts),
        }
    }

    pub fn closed_by_remote(&mut self, ts: Instant<CLK>)
    where
        Generic<CLK::T>: TryInto<Milliseconds>,
    {
        match self {
            Socket::Tcp(s) => s.closed_by_remote(ts),
            Socket::Udp(s) => s.closed_by_remote(ts),
        }
    }

    pub fn set_available_data(&mut self, available_data: usize) {
        match self {
            Socket::Tcp(s) => s.set_available_data(available_data),
            Socket::Udp(s) => s.set_available_data(available_data),
        }
    }

    pub fn rx_enqueue_slice(&mut self, data: &[u8]) -> usize {
        match self {
            Socket::Tcp(s) => s.rx_enqueue_slice(data),
            Socket::Udp(s) => s.rx_enqueue_slice(data),
        }
    }

    pub fn rx_window(&self) -> usize {
        match self {
            Socket::Tcp(s) => s.rx_window(),
            Socket::Udp(s) => s.rx_window(),
        }
    }

    pub fn can_recv(&self) -> bool {
        match self {
            Socket::Tcp(s) => s.can_recv(),
            Socket::Udp(s) => s.can_recv(),
        }
    }
}

/// A conversion trait for network sockets.
pub trait AnySocket<L: ArrayLength<u8>, CLK: Clock>: Sized {
    fn downcast(socket_ref: SocketRef<'_, Socket<L, CLK>>) -> Result<SocketRef<'_, Self>>;
}

#[cfg(feature = "socket-tcp")]
impl<L: ArrayLength<u8>, CLK: Clock> AnySocket<L, CLK> for TcpSocket<L, CLK> {
    fn downcast(ref_: SocketRef<'_, Socket<L, CLK>>) -> Result<SocketRef<'_, Self>> {
        match SocketRef::into_inner(ref_) {
            Socket::Tcp(ref mut socket) => Ok(SocketRef::new(socket)),
            _ => Err(Error::Illegal),
        }
    }
}

#[cfg(feature = "socket-udp")]
impl<L: ArrayLength<u8>, CLK: Clock> AnySocket<L, CLK> for UdpSocket<L, CLK> {
    fn downcast(ref_: SocketRef<'_, Socket<L, CLK>>) -> Result<SocketRef<'_, Self>> {
        match SocketRef::into_inner(ref_) {
            Socket::Udp(ref mut socket) => Ok(SocketRef::new(socket)),
            _ => Err(Error::Illegal),
        }
    }
}
