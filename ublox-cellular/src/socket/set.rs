use super::{AnySocket, Error, Result, Socket, SocketRef, SocketType};

use heapless::{ArrayLength, LinearMap};
use serde::{Deserialize, Serialize};

/// An item of a socket set.
///
/// The only reason this struct is public is to allow the socket set storage
/// to be allocated externally.
pub struct Item<L: ArrayLength<u8>> {
    socket: Socket<L>,
    refs: usize,
}

/// A handle, identifying a socket in a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct Handle(pub usize);

/// An extensible set of sockets.
#[derive(Default)]
pub struct Set<N, L>
where
    N: ArrayLength<(usize, Item<L>)>,
    L: ArrayLength<u8>,
{
    sockets: LinearMap<usize, Item<L>, N>,
}

impl<N, L> Set<N, L>
where
    N: ArrayLength<(usize, Item<L>)>,
    L: ArrayLength<u8>,
{
    /// Create a socket set using the provided storage.
    pub fn new() -> Set<N, L> {
        Set {
            sockets: LinearMap::new(),
        }
    }

    pub fn socket_type(&self, handle: &Handle) -> Option<SocketType> {
        match self.sockets.get(&handle.0) {
            Some(item) => Some(item.socket.get_type()),
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.sockets.len()
    }

    /// Add a socket to the set with the reference count 1, and return its handle.
    pub fn add<T>(&mut self, socket: T) -> Result<Handle>
    where
        T: Into<Socket<L>>,
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
    pub fn get<T: AnySocket<L>>(&mut self, handle: &Handle) -> Result<SocketRef<T>> {
        match self.sockets.get_mut(&handle.0) {
            Some(item) => Ok(T::downcast(SocketRef::new(&mut item.socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Remove a socket from the set, without changing its state.
    pub fn remove(&mut self, handle: Handle) -> Result<Socket<L>> {
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

    /// Iterate every socket in this set.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &Socket<L>)> {
        self.sockets.iter().map(|(k, v)| (Handle(*k), &v.socket))
    }

    // /// Iterate every socket in this set, as SocketRef.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, SocketRef<Socket<L>>)> {
        self.sockets
            .iter_mut()
            .map(|(k, v)| (Handle(*k), SocketRef::new(&mut v.socket)))
    }
}
