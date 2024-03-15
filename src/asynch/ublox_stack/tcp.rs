#![cfg(feature = "dontbuild")]

use core::cell::RefCell;
use core::future::poll_fn;
use core::mem;
use core::task::Poll;

use atat::asynch::AtatClient;
use embedded_nal_async::SocketAddr;
use ublox_sockets::{tcp, SocketHandle, TcpState};

use super::{SocketStack, UbloxStack};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    ConnectionReset,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectError {
    /// The socket is already connected or listening.
    InvalidState,
    /// The remote host rejected the connection with a RST packet.
    ConnectionReset,
    /// Connect timed out.
    TimedOut,
    /// No route to host.
    NoRoute,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AcceptError {
    /// The socket is already connected or listening.
    InvalidState,
    /// Invalid listen port
    InvalidPort,
    /// The remote host rejected the connection with a RST packet.
    ConnectionReset,
}

pub struct TcpSocket<'a> {
    io: TcpIo<'a>,
}

pub struct TcpReader<'a> {
    io: TcpIo<'a>,
}

pub struct TcpWriter<'a> {
    io: TcpIo<'a>,
}

impl<'a> TcpReader<'a> {
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.read(buf).await
    }
}

impl<'a> TcpWriter<'a> {
    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.io.write(buf).await
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        self.io.flush().await
    }
}

impl<'a> TcpSocket<'a> {
    pub fn new<AT: AtatClient, const URC_CAPACITY: usize>(
        stack: &'a UbloxStack<AT, URC_CAPACITY>,
        rx_buffer: &'a mut [u8],
        tx_buffer: &'a mut [u8],
    ) -> Self {
        let s = &mut *stack.socket.borrow_mut();
        let rx_buffer: &'static mut [u8] = unsafe { mem::transmute(rx_buffer) };
        let tx_buffer: &'static mut [u8] = unsafe { mem::transmute(tx_buffer) };
        let handle = s.sockets.add(tcp::Socket::new(
            tcp::SocketBuffer::new(rx_buffer),
            tcp::SocketBuffer::new(tx_buffer),
        ));

        Self {
            io: TcpIo {
                stack: &stack.socket,
                handle,
            },
        }
    }

    pub fn split(&mut self) -> (TcpReader<'_>, TcpWriter<'_>) {
        (TcpReader { io: self.io }, TcpWriter { io: self.io })
    }

    pub async fn connect<T>(&mut self, remote_endpoint: T) -> Result<(), ConnectError>
    where
        T: Into<SocketAddr>,
    {
        match { self.io.with_mut(|s| s.connect(remote_endpoint, None)) } {
            Ok(()) => {}
            Err(_) => return Err(ConnectError::InvalidState),
            // Err(tcp::ConnectError::Unaddressable) => return Err(ConnectError::NoRoute),
        }

        poll_fn(|cx| {
            self.io.with_mut(|s| match s.state() {
                tcp::State::TimeWait => Poll::Ready(Err(ConnectError::ConnectionReset)),
                tcp::State::Listen => unreachable!(),
                tcp::State::Closed | tcp::State::SynSent | tcp::State::SynReceived => {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                }
                _ => Poll::Ready(Ok(())),
            })
        })
        .await
    }

    // FIXME:
    // pub async fn accept<T>(&mut self, local_endpoint: T) -> Result<(), AcceptError>
    // where
    //     T: Into<IpListenEndpoint>,
    // {
    //     match self.io.with_mut(|s, _| s.listen(local_endpoint)) {
    //         Ok(()) => {}
    //         Err(tcp::ListenError::InvalidState) => return Err(AcceptError::InvalidState),
    //         Err(tcp::ListenError::Unaddressable) => return Err(AcceptError::InvalidPort),
    //     }

    //     poll_fn(|cx| {
    //         self.io.with_mut(|s, _| match s.state() {
    //             tcp::State::Listen | tcp::State::SynSent | tcp::State::SynReceived => {
    //                 s.register_send_waker(cx.waker());
    //                 Poll::Pending
    //             }
    //             _ => Poll::Ready(Ok(())),
    //         })
    //     })
    //     .await
    // }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.io.read(buf).await
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.io.write(buf).await
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        self.io.flush().await
    }

    // pub fn set_timeout(&mut self, duration: Option<Duration>) {
    //     self.io.with_mut(|s| s.set_timeout(duration))
    // }

    // pub fn set_keep_alive(&mut self, interval: Option<Duration>) {
    //     self.io.with_mut(|s| s.set_keep_alive(interval))
    // }

    // pub fn local_endpoint(&self) -> Option<IpEndpoint> {
    //     self.io.with(|s, _| s.local_endpoint())
    // }

    pub fn remote_endpoint(&self) -> Option<SocketAddr> {
        self.io.with(|s| s.remote_endpoint())
    }

    pub fn state(&self) -> tcp::State {
        self.io.with(|s| s.state())
    }

    pub fn close(&mut self) {
        self.io.with_mut(|s| s.close())
    }

    pub fn abort(&mut self) {
        self.io.with_mut(|s| s.abort())
    }

    pub fn may_send(&self) -> bool {
        self.io.with(|s| s.may_send())
    }

    pub fn may_recv(&self) -> bool {
        self.io.with(|s| s.may_recv())
    }
}

impl<'a> Drop for TcpSocket<'a> {
    fn drop(&mut self) {
        if matches!(self.state(), TcpState::Listen | TcpState::Established) {
            if let Some(peer_handle) = self.io.with(|s| s.peer_handle) {
                self.io
                    .stack
                    .borrow_mut()
                    .dropped_sockets
                    .push(peer_handle)
                    .ok();
            }
        }
        let mut stack = self.io.stack.borrow_mut();
        stack.sockets.remove(self.io.handle);
        stack.waker.wake();
    }
}

// =======================

#[derive(Copy, Clone)]
struct TcpIo<'a> {
    stack: &'a RefCell<SocketStack>,
    handle: SocketHandle,
}

impl<'d> TcpIo<'d> {
    fn with<R>(&self, f: impl FnOnce(&tcp::Socket) -> R) -> R {
        let s = &*self.stack.borrow();
        let socket = s.sockets.get::<tcp::Socket>(self.handle);
        f(socket)
    }

    fn with_mut<R>(&mut self, f: impl FnOnce(&mut tcp::Socket) -> R) -> R {
        let s = &mut *self.stack.borrow_mut();
        let socket = s.sockets.get_mut::<tcp::Socket>(self.handle);
        let res = f(socket);
        s.waker.wake();
        res
    }

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        poll_fn(move |cx| {
            // CAUTION: smoltcp semantics around EOF are different to what you'd expect
            // from posix-like IO, so we have to tweak things here.
            self.with_mut(|s| match s.recv_slice(buf) {
                // No data ready
                Ok(0) => {
                    s.register_recv_waker(cx.waker());
                    Poll::Pending
                }
                // Data ready!
                Ok(n) => Poll::Ready(Ok(n)),
                // EOF
                Err(_) => Poll::Ready(Ok(0)),
                // FIXME:
                // Err(tcp::RecvError::Finished) => Poll::Ready(Ok(0)),
                // Connection reset. TODO: this can also be timeouts etc, investigate.
                // Err(tcp::RecvError::InvalidState) => Poll::Ready(Err(Error::ConnectionReset)),
            })
        })
        .await
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        poll_fn(move |cx| {
            self.with_mut(|s| match s.send_slice(buf) {
                // Not ready to send (no space in the tx buffer)
                Ok(0) => {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                }
                // Some data sent
                Ok(n) => Poll::Ready(Ok(n)),
                // Connection reset. TODO: this can also be timeouts etc, investigate.
                Err(_) => Poll::Ready(Err(Error::ConnectionReset)),
                // FIXME:
                // Err(tcp::SendError::InvalidState) => Poll::Ready(Err(Error::ConnectionReset)),
            })
        })
        .await
    }

    async fn flush(&mut self) -> Result<(), Error> {
        poll_fn(move |cx| {
            self.with_mut(|s| {
                // If there are outstanding send operations, register for wake up and wait
                // smoltcp issues wake-ups when octets are dequeued from the send buffer
                if s.send_queue() > 0 {
                    s.register_send_waker(cx.waker());
                    Poll::Pending
                // No outstanding sends, socket is flushed
                } else {
                    Poll::Ready(Ok(()))
                }
            })
        })
        .await
    }
}

mod embedded_io_impls {
    use super::*;

    impl embedded_io_async::Error for ConnectError {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            embedded_io_async::ErrorKind::Other
        }
    }

    impl embedded_io_async::Error for Error {
        fn kind(&self) -> embedded_io_async::ErrorKind {
            embedded_io_async::ErrorKind::Other
        }
    }

    impl<'d> embedded_io_async::ErrorType for TcpSocket<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Read for TcpSocket<'d> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.io.read(buf).await
        }
    }

    impl<'d> embedded_io_async::Write for TcpSocket<'d> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.io.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.io.flush().await
        }
    }

    impl<'d> embedded_io_async::ErrorType for TcpReader<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Read for TcpReader<'d> {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.io.read(buf).await
        }
    }

    impl<'d> embedded_io_async::ErrorType for TcpWriter<'d> {
        type Error = Error;
    }

    impl<'d> embedded_io_async::Write for TcpWriter<'d> {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.io.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.io.flush().await
        }
    }
}

// #[cfg(all(feature = "unstable-traits", feature = "nightly"))]
pub mod client {
    use core::cell::UnsafeCell;
    use core::mem::MaybeUninit;
    use core::ptr::NonNull;

    use atomic_polyfill::{AtomicBool, Ordering};

    use super::*;

    /// TCP client capable of creating up to N multiple connections with tx and rx buffers according to TX_SZ and RX_SZ.
    pub struct TcpClient<
        'd,
        AT: AtatClient + 'static,
        const N: usize,
        const URC_CAPACITY: usize,
        const TX_SZ: usize = 1024,
        const RX_SZ: usize = 1024,
    > {
        pub(crate) stack: &'d UbloxStack<AT, URC_CAPACITY>,
        pub(crate) state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
    }

    impl<
            'd,
            AT: AtatClient,
            const N: usize,
            const URC_CAPACITY: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > TcpClient<'d, AT, N, URC_CAPACITY, TX_SZ, RX_SZ>
    {
        /// Create a new TcpClient
        pub fn new(
            stack: &'d UbloxStack<AT, URC_CAPACITY>,
            state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        ) -> Self {
            Self { stack, state }
        }
    }

    impl<
            'd,
            AT: AtatClient,
            const N: usize,
            const URC_CAPACITY: usize,
            const TX_SZ: usize,
            const RX_SZ: usize,
        > embedded_nal_async::TcpConnect for TcpClient<'d, AT, N, URC_CAPACITY, TX_SZ, RX_SZ>
    {
        type Error = Error;
        type Connection<'m> = TcpConnection<'m, N, TX_SZ, RX_SZ> where Self: 'm;

        async fn connect<'a>(
            &'a self,
            remote: embedded_nal_async::SocketAddr,
        ) -> Result<Self::Connection<'a>, Self::Error>
        where
            Self: 'a,
        {
            let remote_endpoint = (remote.ip(), remote.port());
            let mut socket = TcpConnection::new(&self.stack, self.state)?;
            socket
                .socket
                .connect(remote_endpoint)
                .await
                .map_err(|_| Error::ConnectionReset)?;
            Ok(socket)
        }
    }

    pub struct TcpConnection<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
        socket: TcpSocket<'d>,
        state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        bufs: NonNull<([u8; TX_SZ], [u8; RX_SZ])>,
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize>
        TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        fn new<AT: AtatClient, const URC_CAPACITY: usize>(
            stack: &'d UbloxStack<AT, URC_CAPACITY>,
            state: &'d TcpClientState<N, TX_SZ, RX_SZ>,
        ) -> Result<Self, Error> {
            let mut bufs = state.pool.alloc().ok_or(Error::ConnectionReset)?;
            Ok(Self {
                socket: unsafe {
                    TcpSocket::new(stack, &mut bufs.as_mut().1, &mut bufs.as_mut().0)
                },
                state,
                bufs,
            })
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> Drop
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        fn drop(&mut self) {
            unsafe {
                self.socket.close();
                self.state.pool.free(self.bufs);
            }
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::ErrorType
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        type Error = Error;
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Read
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
            self.socket.read(buf).await
        }
    }

    impl<'d, const N: usize, const TX_SZ: usize, const RX_SZ: usize> embedded_io_async::Write
        for TcpConnection<'d, N, TX_SZ, RX_SZ>
    {
        async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
            self.socket.write(buf).await
        }

        async fn flush(&mut self) -> Result<(), Self::Error> {
            self.socket.flush().await
        }
    }

    /// State for TcpClient
    pub struct TcpClientState<const N: usize, const TX_SZ: usize, const RX_SZ: usize> {
        pool: Pool<([u8; TX_SZ], [u8; RX_SZ]), N>,
    }

    impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> TcpClientState<N, TX_SZ, RX_SZ> {
        pub const fn new() -> Self {
            Self { pool: Pool::new() }
        }
    }

    unsafe impl<const N: usize, const TX_SZ: usize, const RX_SZ: usize> Sync
        for TcpClientState<N, TX_SZ, RX_SZ>
    {
    }

    struct Pool<T, const N: usize> {
        used: [AtomicBool; N],
        data: [UnsafeCell<MaybeUninit<T>>; N],
    }

    impl<T, const N: usize> Pool<T, N> {
        const VALUE: AtomicBool = AtomicBool::new(false);
        const UNINIT: UnsafeCell<MaybeUninit<T>> = UnsafeCell::new(MaybeUninit::uninit());

        const fn new() -> Self {
            Self {
                used: [Self::VALUE; N],
                data: [Self::UNINIT; N],
            }
        }
    }

    impl<T, const N: usize> Pool<T, N> {
        fn alloc(&self) -> Option<NonNull<T>> {
            for n in 0..N {
                if self.used[n].swap(true, Ordering::SeqCst) == false {
                    let p = self.data[n].get() as *mut T;
                    return Some(unsafe { NonNull::new_unchecked(p) });
                }
            }
            None
        }

        /// safety: p must be a pointer obtained from self.alloc that hasn't been freed yet.
        unsafe fn free(&self, p: NonNull<T>) {
            let origin = self.data.as_ptr() as *mut T;
            let n = p.as_ptr().offset_from(origin);
            assert!(n >= 0);
            assert!((n as usize) < N);
            self.used[n as usize].store(false, Ordering::SeqCst);
        }
    }
}
