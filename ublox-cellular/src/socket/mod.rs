use core::marker::PhantomData;

mod meta;
mod ref_;
mod ring_buffer;
mod set;
pub mod tcp;
pub mod udp;

pub(crate) use self::meta::Meta as SocketMeta;
pub use self::ring_buffer::RingBuffer;
use heapless::ArrayLength;

#[cfg(feature = "socket-tcp")]
pub use tcp::{State as TcpState, TcpSocket};
#[cfg(feature = "socket-udp")]
pub use udp::UdpSocket;

pub use self::set::{Handle as SocketHandle, Item as SocketSetItem, Set as SocketSet};

pub use self::ref_::Ref as SocketRef;
pub(crate) use self::ref_::Session as SocketSession;

/// The error type for the networking stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An operation cannot proceed because a buffer is empty or full.
    Exhausted,
    /// An operation is not permitted in the current state.
    Illegal,
    /// An endpoint or address of a remote host could not be translated to a lower level address.
    /// E.g. there was no an Ethernet address corresponding to an IPv4 address in the ARP cache,
    /// or a TCP connection attempt was made to an unspecified endpoint.
    Unaddressable,

    /// An incoming packet could not be parsed because some of its fields were out of bounds
    /// of the received data.
    Truncated,
    /// An incoming packet had an incorrect checksum and was dropped.
    Checksum,
    /// An incoming packet could not be recognized and was dropped.
    /// E.g. an Ethernet packet with an unknown EtherType.
    Unrecognized,
    /// An incoming IP packet has been split into several IP fragments and was dropped,
    /// since IP reassembly is not supported.
    Fragmented,
    /// An incoming packet was recognized but was self-contradictory.
    /// E.g. a TCP packet with both SYN and FIN flags set.
    Malformed,
    /// An incoming packet was recognized but contradicted internal state.
    /// E.g. a TCP packet addressed to a socket that doesn't exist.
    Dropped,

    SocketSetFull,
    InvalidSocket,

    #[doc(hidden)]
    __Nonexhaustive,
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
pub enum Socket<L: ArrayLength<u8>> {
    // #[cfg(feature = "socket-raw")]
    // Raw(RawSocket<'a, 'b>),
    // #[cfg(all(
    //     feature = "socket-icmp",
    //     any(feature = "proto-ipv4", feature = "proto-ipv6")
    // ))]
    // Icmp(IcmpSocket<'a, 'b>),
    #[cfg(feature = "socket-udp")]
    Udp(UdpSocket<L>),
    #[cfg(feature = "socket-tcp")]
    Tcp(TcpSocket<L>),
    #[doc(hidden)]
    __Nonexhaustive(PhantomData<()>),
}

#[derive(Debug)]
pub enum SocketType {
    Udp,
    Tcp,
    #[doc(hidden)]
    __Nonexhaustive,
}

impl<L: ArrayLength<u8>> Socket<L> {
    pub fn get_type(&self) -> SocketType {
        match self {
            Socket::Tcp(_) => SocketType::Tcp,
            Socket::Udp(_) => SocketType::Udp,
            Socket::__Nonexhaustive(_) => SocketType::__Nonexhaustive,
        }
    }
}

macro_rules! dispatch_socket {
    ($self_:expr, |$socket:ident| $code:expr) => {
        dispatch_socket!(@inner $self_, |$socket| $code);
    };
    (mut $self_:expr, |$socket:ident| $code:expr) => {
        dispatch_socket!(@inner mut $self_, |$socket| $code);
    };
    (@inner $( $mut_:ident )* $self_:expr, |$socket:ident| $code:expr) => {
        match *$self_ {
            // #[cfg(feature = "socket-raw")]
            // Socket::Raw(ref $( $mut_ )* $socket) => $code,
            // #[cfg(all(feature = "socket-icmp", any(feature = "proto-ipv4", feature = "proto-ipv6")))]
            // Socket::Icmp(ref $( $mut_ )* $socket) => $code,
            #[cfg(feature = "socket-udp")]
            Socket::Udp(ref $( $mut_ )* $socket) => $code,
            #[cfg(feature = "socket-tcp")]
            Socket::Tcp(ref $( $mut_ )* $socket) => $code,
            Socket::__Nonexhaustive(_) => unreachable!()
        }
    };
}

impl<L: ArrayLength<u8>> Socket<L> {
    /// Return the socket handle.
    #[inline]
    pub fn handle(&self) -> SocketHandle {
        self.meta().handle
    }

    pub(crate) fn meta(&self) -> &SocketMeta {
        dispatch_socket!(self, |socket| &socket.meta)
    }

    // pub(crate) fn meta_mut(&mut self) -> &mut SocketMeta {
    //     dispatch_socket!(mut self, |socket| &mut socket.meta)
    // }
}

impl<L: ArrayLength<u8>> SocketSession for Socket<L> {
    fn finish(&mut self) {
        dispatch_socket!(mut self, |socket| socket.finish())
    }
}

/// A conversion trait for network sockets.
pub trait AnySocket<L: ArrayLength<u8>>: SocketSession + Sized {
    fn downcast(socket_ref: SocketRef<'_, Socket<L>>) -> Result<SocketRef<'_, Self>>;
}

/// A trait for setting a value to a known state.
///
/// In-place analog of Default.
pub trait Resettable {
    fn reset(&mut self);
}

macro_rules! from_socket {
    ($socket:ty, $variant:ident) => {
        impl<L: ArrayLength<u8>> AnySocket<L> for $socket {
            fn downcast(ref_: SocketRef<'_, Socket<L>>) -> Result<SocketRef<'_, Self>> {
                match SocketRef::into_inner(ref_) {
                    Socket::$variant(ref mut socket) => Ok(SocketRef::new(socket)),
                    _ => Err(Error::Illegal),
                }
            }
        }
    };
}

// #[cfg(feature = "socket-raw")]
// from_socket!(RawSocket, Raw);
// #[cfg(all(
//     feature = "socket-icmp",
//     any(feature = "proto-ipv4", feature = "proto-ipv6")
// ))]
// from_socket!(IcmpSocket, Icmp);
#[cfg(feature = "socket-udp")]
from_socket!(UdpSocket<L>, Udp);
#[cfg(feature = "socket-tcp")]
from_socket!(TcpSocket<L>, Tcp);
