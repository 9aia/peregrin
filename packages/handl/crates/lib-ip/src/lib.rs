//! # handl-lib-ip
//!
//! IP address, CIDR, and socket utilities for the handl runtime.
//!
//! Provides ergonomic, zero-allocation-by-default wrappers around IPv4,
//! IPv6, CIDR prefixes, and socket addresses, with strict validation and
//! lossless round-trip parsing.
//!
//! ## Usage
//!
//! ```ignore
//! use handl_lib_ip::{IpAddr, Ipv4, Ipv6, Cidr, SocketAddr};
//!
//! let v4 = Ipv4::parse("192.168.1.1")?;
//! let v6 = Ipv6::parse("::1")?;
//! let cidr = Cidr::parse("10.0.0.0/8")?;
//! let sock = SocketAddr::parse("[::1]:8080")?;
//! ```

mod addr;
mod cidr;
mod error;
mod socket;

#[cfg(test)]
mod tests;

pub use addr::{IpAddr, Ipv4, Ipv6};
pub use cidr::{Cidr, Cidr4, Cidr6};
pub use error::IpError;
pub use socket::SocketAddr;
