use super::{AnySocket, Error, Result, Socket, SocketRef, SocketType};

use heapless::{ArrayLength, Vec};
use serde::{Deserialize, Serialize};

/// An item of a socket set.
///
/// The only reason this struct is public is to allow the socket set storage
/// to be allocated externally.
pub struct Item<L: ArrayLength<u8>> {
    socket: Socket<L>,
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

        self.sockets
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(Error::SocketSetFull)?
            .replace(Item { socket });

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
    pub fn remove(&mut self, handle: Handle) -> Result<Item<L>> {
        let index = self.index_of(handle)?;
        let item: &mut Option<Item<L>> =
            self.sockets.get_mut(index).ok_or(Error::InvalidSocket)?;

        item.take().ok_or(Error::InvalidSocket)
    }

    /// Prune the sockets in this set.
    ///
    /// Pruning affects sockets with reference count 0. Open sockets are closed.
    /// Closed sockets are removed and dropped.
    pub fn prune(&mut self) {
        self.sockets.iter_mut().for_each(|slot| {
            slot.take();
        })
    }

    /// Iterate every socket in this set.
    pub fn iter(&self) -> impl Iterator<Item = (Handle, &Socket<L>)> {
        self.sockets.iter().filter_map(|slot| {
            if let Some(Item { ref socket }) = slot {
                Some((Handle(socket.handle().0), socket))
            } else {
                None
            }
        })
    }

    /// Iterate every socket in this set, as SocketRef.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle, SocketRef<Socket<L>>)> {
        self.sockets.iter_mut().filter_map(|slot| {
            if let Some(Item { ref mut socket }) = slot {
                Some((Handle(socket.handle().0), SocketRef::new(socket)))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::sockets::{TcpSocket, UdpSocket};

    use super::*;
    use heapless::consts;

    #[test]
    fn add_socket() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn remove_socket() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);

        assert!(set.remove(Handle(0)).is_ok());
        assert_eq!(set.len(), 1);

        assert!(set.get::<TcpSocket<_>>(Handle(0)).is_err());

        set.get::<UdpSocket<_>>(Handle(1))
            .expect("failed to get udp socket");
    }

    #[test]
    fn add_duplicate_socket() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(0)), Err(Error::DuplicateSocket));
    }

    #[test]
    fn add_socket_to_full_set() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);
        assert_eq!(set.add(UdpSocket::new(2)), Err(Error::SocketSetFull));
    }

    #[test]
    fn get_socket() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);

        set.get::<TcpSocket<_>>(Handle(0))
            .expect("failed to get tcp socket");

        set.get::<UdpSocket<_>>(Handle(1))
            .expect("failed to get udp socket");
    }

    #[test]
    fn get_socket_wrong_type() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);

        assert!(set.get::<TcpSocket<_>>(Handle(1)).is_err());

        set.get::<UdpSocket<_>>(Handle(1))
            .expect("failed to get udp socket");
    }

    #[test]
    fn get_socket_type() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);

        assert_eq!(set.socket_type(Handle(0)), Some(SocketType::Tcp));
        assert_eq!(set.socket_type(Handle(1)), Some(SocketType::Udp));
    }

    #[test]
    fn replace_socket() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);

        assert!(set.remove(Handle(0)).is_ok());
        assert_eq!(set.len(), 1);

        assert!(set.get::<TcpSocket<_>>(Handle(0)).is_err());

        set.get::<UdpSocket<_>>(Handle(1))
            .expect("failed to get udp socket");

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 2);

        set.get::<TcpSocket<_>>(Handle(0))
            .expect("failed to get tcp socket");
    }

    #[test]
    fn prune_socket_set() {
        let mut set = Set::<consts::U2, consts::U64>::new();

        assert_eq!(set.add(TcpSocket::new(0)), Ok(Handle(0)));
        assert_eq!(set.len(), 1);
        assert_eq!(set.add(UdpSocket::new(1)), Ok(Handle(1)));
        assert_eq!(set.len(), 2);

        set.get::<TcpSocket<_>>(Handle(0))
            .expect("failed to get tcp socket");

        set.prune();
        assert_eq!(set.len(), 0);
    }
}
