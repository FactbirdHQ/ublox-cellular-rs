use core::{fmt, slice};

use super::{AnySocket, Error, Result, Socket, SocketRef};

use heapless::{ArrayLength, LinearMap};
use serde::{Deserialize, Serialize};

/// An item of a socket set.
///
/// The only reason this struct is public is to allow the socket set storage
/// to be allocated externally.
#[derive(Debug)]
pub struct Item {
    socket: Socket,
    refs: usize,
}

/// A handle, identifying a socket in a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Handle(pub usize);

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An extensible set of sockets.
#[derive(Debug, Default)]
pub struct Set<N>
where
    N: ArrayLength<(usize, Item)>,
{
    sockets: LinearMap<usize, Item, N>,
}

impl<N> Set<N>
where
    N: ArrayLength<(usize, Item)>,
{
    /// Create a socket set using the provided storage.
    pub fn new() -> Set<N> {
        Set {
            sockets: LinearMap::new(),
        }
    }

    /// Add a socket to the set with the reference count 1, and return its handle.
    pub fn add<T>(&mut self, socket: T) -> Result<Handle>
    where
        T: Into<Socket>,
    {
        let socket = socket.into();
        let handle = socket.handle();
        match self
            .sockets
            .insert(socket.handle().0, Item { socket, refs: 1 })
        {
            Ok(_) => Ok(handle),
            _ => Err(Error::SocketSetFull),
        }
    }

    /// Get a socket from the set by its handle, as mutable.
    pub fn get<T: AnySocket>(&mut self, handle: &Handle) -> Result<SocketRef<T>> {
        match self.sockets.get_mut(&handle.0) {
            Some(item) => Ok(T::downcast(SocketRef::new(&mut item.socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Remove a socket from the set, without changing its state.
    pub fn remove(&mut self, handle: Handle) -> Result<Socket> {
        // net_trace!("[{}]: removing", handle.0);
        match self.sockets.remove(&handle.0) {
            Some(item) => Ok(item.socket),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Increase reference count by 1.
    pub fn retain(&mut self, handle: Handle) -> Result<()> {
        match self.sockets.get_mut(&handle.0) {
            Some(v) => v.refs += 1,
            None => return Err(Error::InvalidSocket),
        };
        Ok(())
    }

    /// Decrease reference count by 1.
    pub fn release(&mut self, handle: Handle) -> Result<()> {
        match self.sockets.get_mut(&handle.0) {
            Some(v) => {
                if v.refs == 0 {
                    return Err(Error::Illegal);
                }
                v.refs -= 1;
                Ok(())
            }
            None => Err(Error::InvalidSocket),
        }
    }

    // /// Prune the sockets in this set.
    // ///
    // /// Pruning affects sockets with reference count 0. Open sockets are closed.
    // /// Closed sockets are removed and dropped.
    // pub fn prune(&mut self) {
    //     for (_, item) in self.sockets.iter_mut() {
    //         let mut may_remove = false;
    //         if let Item {
    //             refs: 0,
    //             ref mut socket,
    //         } = item
    //         {
    //             match *socket {
    //                 #[cfg(feature = "socket-raw")]
    //                 Socket::Raw(_) => may_remove = true,
    //                 #[cfg(all(
    //                     feature = "socket-icmp",
    //                     any(feature = "proto-ipv4", feature = "proto-ipv6")
    //                 ))]
    //                 Socket::Icmp(_) => may_remove = true,
    //                 #[cfg(feature = "socket-udp")]
    //                 Socket::Udp(_) => may_remove = true,
    //                 #[cfg(feature = "socket-tcp")]
    //                 Socket::Tcp(ref mut socket) => {
    //                     if socket.state() == TcpState::Closed {
    //                         may_remove = true
    //                     } else {
    //                         socket.close()
    //                     }
    //                 }
    //                 Socket::__Nonexhaustive(_) => unreachable!(),
    //             }
    //         }
    //         if may_remove {
    //             // net_trace!("[{}]: pruning", index);
    //             *item = None
    //         }
    //     }
    // }

    // / Iterate every socket in this set.
    // pub fn iter(&self) -> Iter {
    //     Iter {
    //         lower: self.sockets.iter(),
    //     }
    // }

    // /// Iterate every socket in this set, as SocketRef.
    // pub fn iter_mut(&mut self) -> IterMut {
    //     IterMut {
    //         lower: self.sockets.iter_mut(),
    //     }
    // }
}

/// Immutable socket set iterator.
///
/// This struct is created by the [iter](struct.SocketSet.html#method.iter)
/// on [socket sets](struct.SocketSet.html).
pub struct Iter<'a> {
    lower: slice::Iter<'a, Option<Item>>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Socket;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item_opt) = self.lower.next() {
            if let Some(item) = item_opt.as_ref() {
                return Some(&item.socket);
            }
        }
        None
    }
}

/// Mutable socket set iterator.
///
/// This struct is created by the [iter_mut](struct.SocketSet.html#method.iter_mut)
/// on [socket sets](struct.SocketSet.html).
pub struct IterMut<'a> {
    lower: slice::IterMut<'a, Option<Item>>,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = SocketRef<'a, Socket>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item_opt) = self.lower.next() {
            if let Some(item) = item_opt.as_mut() {
                return Some(SocketRef::new(&mut item.socket));
            }
        }
        None
    }
}
