use core::ops::{Deref, DerefMut};

/// A smart pointer to a socket.
///
/// Allows the network stack to efficiently determine if the socket state was changed in any way.
pub struct Ref<'a, T: 'a> {
    socket: &'a mut T,
    consumed: bool,
}

impl<'a, T: 'a> Ref<'a, T> {
    /// Wrap a pointer to a socket to make a smart pointer.
    ///
    /// Calling this function is only necessary if your code is using [into_inner].
    ///
    /// [into_inner]: #method.into_inner
    pub fn new(socket: &'a mut T) -> Self {
        Ref {
            socket,
            consumed: false,
        }
    }

    /// Unwrap a smart pointer to a socket.
    ///
    /// The finalization code is not run. Prompt operation of the network stack depends
    /// on wrapping the returned pointer back and dropping it.
    ///
    /// Calling this function is only necessary to achieve composability if you *must*
    /// map a `&mut SocketRef<'a, XSocket>` to a `&'a mut XSocket` (note the lifetimes);
    /// be sure to call [new] afterwards.
    ///
    /// [new]: #method.new_unchecked
    pub fn into_inner(mut ref_: Self) -> &'a mut T {
        ref_.consumed = true;
        ref_.socket
    }
}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.socket
    }
}

impl<'a, T> DerefMut for Ref<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.socket
    }
}
