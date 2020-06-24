use super::{AnySocket, Error, Result, Socket, SocketRef, SocketType};

use heapless::{ArrayLength, Vec};
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
    N: ArrayLength<Option<Item<L>>>,
    L: ArrayLength<u8>,
{
    pub sockets: Vec<Option<Item<L>>, N>,
}

impl<N, L> Set<N, L>
where
    N: ArrayLength<Option<Item<L>>>,
    L: ArrayLength<u8>,
{
    /// Create a socket set using the provided storage.
    pub fn new() -> Set<N, L> {
        let mut sockets = Vec::new();
        while sockets.len() < N::to_usize() {
            sockets.push(None).ok();
        }
        Set { sockets }
    }

    /// Get the maximum number of sockets the set can hold
    pub fn capacity(&self) -> usize {
        N::to_usize()
    }

    /// Get the current number of initialized sockets, the set is holding
    pub fn len(&self) -> usize {
        self.sockets.iter().filter(|a| a.is_some()).count()
    }

    /// Get the type of a specific socket in the set.
    ///
    /// Returned as a [`SocketType`]
    pub fn socket_type(&self, handle: Handle) -> Option<SocketType> {
        match self.sockets.iter().find_map(|i| {
            if let Some(ref s) = i {
                if s.socket.handle().0 == handle.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(item) => Some(item.socket.get_type()),
            None => None,
        }
    }

    /// Add a socket to the set with the reference count 1, and return its handle.
    pub fn add<T>(&mut self, socket: T) -> Result<Handle>
    where
        T: Into<Socket<L>>,
    {
        let socket = socket.into();
        for slot in self.sockets.iter_mut() {
            if slot.is_none() {
                let handle = socket.handle();
                *slot = Some(Item { socket, refs: 1 });
                return Ok(handle)
            }
        }
        Err(Error::SocketSetFull)
    }

    /// Get a socket from the set by its handle, as mutable.
    pub fn get<T: AnySocket<L>>(&mut self, handle: Handle) -> Result<SocketRef<T>> {
        match self.sockets.iter_mut().find_map(|i| {
            if let Some(ref mut s) = i {
                if s.socket.handle().0 == handle.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(item) => Ok(T::downcast(SocketRef::new(&mut item.socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    /// Remove a socket from the set, without changing its state.
    pub fn remove(&mut self, handle: Handle) -> Result<Socket<L>> {
        let index = self
            .sockets
            .iter_mut()
            .position(|i| {
                if let Some(s) = i {
                    return s.socket.handle().0 == handle.0;
                }
                false
            }).ok_or(Error::InvalidSocket)?;

        let item: &mut Option<Item<L>> = unsafe { self.sockets.get_unchecked_mut(index) };

        item.take().ok_or(Error::InvalidSocket).map(|item| item.socket)
    }

    /// Increase reference count by 1.
    pub fn retain(&mut self, handle: Handle) -> Result<()> {
        match self.sockets.iter_mut().find_map(|i| {
            if let Some(ref mut s) = i {
                if s.socket.handle().0 == handle.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            Some(v) => v.refs += 1,
            None => return Err(Error::InvalidSocket),
        };
        Ok(())
    }

    /// Decrease reference count by 1.
    pub fn release(&mut self, handle: Handle) -> Result<()> {
        match self.sockets.iter_mut().find_map(|i| {
            if let Some(ref mut s) = i {
                if s.socket.handle().0 == handle.0 {
                    Some(s)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
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
    //             }
    //         }
    //         if may_remove {
    //             *item = None
    //         }
    //     }
    // }

    /// Iterate every socket in this set.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &Socket<L>)> {
        self.sockets.iter().filter_map(|i| {
            if let Some(Item { ref socket, .. }) = i {
                Some((Handle(socket.handle().0), socket))
            } else {
                None
            }
        })
    }

    /// Iterate every socket in this set, as SocketRef.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, SocketRef<Socket<L>>)> {
        self.sockets.iter_mut().filter_map(|i| {
            if let Some(Item { ref mut socket, .. }) = i {
                Some((Handle(socket.handle().0), SocketRef::new(socket)))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {}
