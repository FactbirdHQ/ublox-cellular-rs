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
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    defmt::Format,
)]
pub struct Handle(pub u8);

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

    /// Check if the set is currently holding no active sockets
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the type of a specific socket in the set.
    ///
    /// Returned as a [`SocketType`]
    pub fn socket_type(&self, handle: Handle) -> Option<SocketType> {
        if let Ok(index) = self.index_of(handle) {
            if let Some(Some(item)) = self.sockets.get(index) {
                return Some(item.socket.get_type());
            }
        }
        None
    }

    /// Add a socket to the set with the reference count 1, and return its handle.
    pub fn add<T>(&mut self, socket: T) -> Result<Handle>
    where
        T: Into<Socket<L>>,
    {
        let socket = socket.into();
        let handle = socket.handle();

        if self.index_of(handle).is_ok() {
            return Err(Error::DuplicateSocket);
        }

        let slot = self
            .sockets
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(Error::SocketSetFull)?;

        *slot = Some(Item { socket, refs: 1 });
        Ok(handle)
    }

    /// Get a socket from the set by its handle, as mutable.
    pub fn get<T: AnySocket<L>>(&mut self, handle: Handle) -> Result<SocketRef<T>> {
        let index = self.index_of(handle)?;

        match self.sockets.get_mut(index).ok_or(Error::InvalidSocket)? {
            Some(item) => Ok(T::downcast(SocketRef::new(&mut item.socket))?),
            None => Err(Error::InvalidSocket),
        }
    }

    pub fn index_of(&self, handle: Handle) -> Result<usize> {
        self.sockets
            .iter()
            .position(|i| {
                i.as_ref()
                    .map(|s| s.socket.handle().0 == handle.0)
                    .unwrap_or(false)
            })
            .ok_or(Error::InvalidSocket)
    }

    /// Remove a socket from the set, without changing its state.
    pub fn remove(&mut self, handle: Handle) -> Result<Socket<L>> {
        let index = self.index_of(handle)?;
        let item: &mut Option<Item<L>> = self.sockets.get_mut(index).ok_or(Error::InvalidSocket)?;

        item.take()
            .ok_or(Error::InvalidSocket)
            .map(|item| item.socket)
    }

    /// Increase reference count by 1.
    pub fn retain(&mut self, handle: Handle) -> Result<()> {
        let index = self.index_of(handle)?;
        match self.sockets.get_mut(index).ok_or(Error::InvalidSocket)? {
            Some(item) => {
                item.refs += 1;
                Ok(())
            }
            None => Err(Error::InvalidSocket),
        }
    }

    /// Decrease reference count by 1.
    pub fn release(&mut self, handle: Handle) -> Result<()> {
        let index = self.index_of(handle)?;
        match self.sockets.get_mut(index).ok_or(Error::InvalidSocket)? {
            Some(v) => {
                v.refs = v.refs.checked_sub(1).ok_or(Error::Illegal)?;
                Ok(())
            }
            None => Err(Error::InvalidSocket),
        }
    }

    /// Prune the sockets in this set.
    ///
    /// Pruning affects sockets with reference count 0. Open sockets are closed.
    /// Closed sockets are removed and dropped.
    pub fn prune(&mut self) {
        self.sockets.iter_mut().for_each(|item| {
            item.take();
        })
    }

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
