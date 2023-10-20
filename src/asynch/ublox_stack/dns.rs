#![cfg(feature = "dontbuild")]


use core::{cell::RefCell, future::poll_fn, task::Poll};

use atat::asynch::AtatClient;
use embedded_nal_async::AddrType;
use no_std_net::IpAddr;

use crate::asynch::ublox_stack::DnsState;

use super::{DnsQuery, SocketStack, UbloxStack};

/// Errors returned by DnsSocket.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// Invalid name
    InvalidName,
    /// Name too long
    NameTooLong,
    /// Name lookup failed
    Failed,
}

/// DNS client compatible with the `embedded-nal-async` traits.
///
/// This exists only for compatibility with crates that use `embedded-nal-async`.
/// Prefer using [`Stack::dns_query`](crate::Stack::dns_query) directly if you're
/// not using `embedded-nal-async`.
pub struct DnsSocket<'a> {
    stack: &'a RefCell<SocketStack>,
}

impl<'a> DnsSocket<'a> {
    /// Create a new DNS socket using the provided stack.
    pub fn new<AT: AtatClient, const URC_CAPACITY: usize>(
        stack: &'a UbloxStack<AT, URC_CAPACITY>,
    ) -> Self {
        Self {
            stack: &stack.socket,
        }
    }

    /// Make a query for a given name and return the corresponding IP addresses.
    pub async fn query(&self, name: &str, addr_type: AddrType) -> Result<IpAddr, Error> {
        match addr_type {
            AddrType::IPv4 => {
                if let Ok(ip) = name.parse().map(IpAddr::V4) {
                    return Ok(ip);
                }
            }
            AddrType::IPv6 => {
                if let Ok(ip) = name.parse().map(IpAddr::V6) {
                    return Ok(ip);
                }
            }
            _ => {}
        }

        {
            let mut s = self.stack.borrow_mut();
            if s.dns_queries
                .insert(heapless::String::from(name), DnsQuery::new())
                .is_err()
            {
                error!(
                    "Attempted to start more simultaneous DNS requests than the (4) supported"
                );
            }
            s.waker.wake();
        }

        #[must_use = "to delay the drop handler invocation to the end of the scope"]
        struct OnDrop<F: FnOnce()> {
            f: core::mem::MaybeUninit<F>,
        }

        impl<F: FnOnce()> OnDrop<F> {
            fn new(f: F) -> Self {
                Self {
                    f: core::mem::MaybeUninit::new(f),
                }
            }

            fn defuse(self) {
                core::mem::forget(self)
            }
        }

        impl<F: FnOnce()> Drop for OnDrop<F> {
            fn drop(&mut self) {
                unsafe { self.f.as_ptr().read()() }
            }
        }

        let drop = OnDrop::new(|| {
            let mut s = self.stack.borrow_mut();
            s.dns_queries.remove(&heapless::String::from(name));
        });

        let res = poll_fn(|cx| {
            let mut s = self.stack.borrow_mut();
            let query = s
                .dns_queries
                .get_mut(&heapless::String::from(name))
                .unwrap();
            match query.state {
                DnsState::Ok(ip) => {
                    s.dns_queries.remove(&heapless::String::from(name));
                    return Poll::Ready(Ok(ip));
                }
                DnsState::Err => {
                    s.dns_queries.remove(&heapless::String::from(name));
                    return Poll::Ready(Err(Error::Failed));
                }
                _ => {
                    query.waker.register(cx.waker());
                    Poll::Pending
                }
            }
        })
        .await;

        drop.defuse();

        res
    }
}

impl<'a> embedded_nal_async::Dns for DnsSocket<'a> {
    type Error = Error;

    async fn get_host_by_name(
        &self,
        host: &str,
        addr_type: AddrType,
    ) -> Result<IpAddr, Self::Error> {
        self.query(host, addr_type).await
    }

    async fn get_host_by_address(
        &self,
        _addr: embedded_nal_async::IpAddr,
    ) -> Result<heapless::String<256>, Self::Error> {
        unimplemented!()
    }
}
