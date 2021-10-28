#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate alloc;

pub mod url;
pub mod req;
pub mod resp;
pub mod stack;

#[cfg(feature="std")]
pub mod std;

pub mod addr {
    pub use embedded_nal::{Ipv4Addr, IpAddr, SocketAddr, SocketAddrV4};
}