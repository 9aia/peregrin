use crate::{IpAddr, IpError, Ipv4, Ipv6};
use core::fmt;

/// An IPv4 CIDR prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cidr4 {
    pub addr: Ipv4,
    pub prefix: u8,
}

impl Cidr4 {
    /// Construct a CIDR from an address and a prefix length.
    pub fn new(addr: Ipv4, prefix: u8) -> Result<Self, IpError> {
        if prefix > 32 {
            return Err(IpError::InvalidCidr);
        }
        Ok(Cidr4 { addr, prefix })
    }

    /// Parse a string such as `192.168.0.0/24`.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        let (addr_part, prefix_part) = s.split_once('/').ok_or(IpError::InvalidCidr)?;
        let addr = Ipv4::parse(addr_part)?;
        let prefix: u8 = prefix_part.parse().map_err(|_| IpError::InvalidCidr)?;
        Self::new(addr, prefix)
    }

    /// Network address for this CIDR.
    pub fn network(self) -> Ipv4 {
        let mask = cidr_mask_v4(self.prefix);
        Ipv4::from_u32(self.addr.to_u32() & mask)
    }

    /// Broadcast address for this CIDR.
    pub fn broadcast(self) -> Ipv4 {
        let mask = cidr_mask_v4(self.prefix);
        Ipv4::from_u32(self.addr.to_u32() | !mask)
    }

    /// Total number of addresses in the CIDR.
    pub fn address_count(self) -> u64 {
        1u64 << (32 - self.prefix as u32)
    }

    /// Number of usable host addresses (subtracts network and broadcast).
    pub fn host_count(self) -> u64 {
        let total = self.address_count();
        if self.prefix >= 31 {
            0
        } else {
            total - 2
        }
    }

    /// First usable host address.
    pub fn first_host(self) -> Ipv4 {
        if self.prefix >= 31 {
            self.network()
        } else {
            Ipv4::from_u32(self.network().to_u32() + 1)
        }
    }

    /// Last usable host address.
    pub fn last_host(self) -> Ipv4 {
        if self.prefix >= 31 {
            self.broadcast()
        } else {
            Ipv4::from_u32(self.broadcast().to_u32() - 1)
        }
    }

    /// Whether the CIDR contains the given address.
    pub fn contains(self, ip: Ipv4) -> bool {
        let mask = cidr_mask_v4(self.prefix);
        (self.addr.to_u32() & mask) == (ip.to_u32() & mask)
    }
}

impl fmt::Display for Cidr4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

/// An IPv6 CIDR prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cidr6 {
    pub addr: Ipv6,
    pub prefix: u8,
}

impl Cidr6 {
    /// Construct a CIDR from an address and a prefix length.
    pub fn new(addr: Ipv6, prefix: u8) -> Result<Self, IpError> {
        if prefix > 128 {
            return Err(IpError::InvalidCidr);
        }
        Ok(Cidr6 { addr, prefix })
    }

    /// Parse a string such as `2001:db8::/32`.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        let (addr_part, prefix_part) = s.split_once('/').ok_or(IpError::InvalidCidr)?;
        let addr = Ipv6::parse(addr_part)?;
        let prefix: u8 = prefix_part.parse().map_err(|_| IpError::InvalidCidr)?;
        Self::new(addr, prefix)
    }

    /// Network address for this CIDR.
    pub fn network(self) -> Ipv6 {
        let mask = cidr_mask_v6(self.prefix);
        Ipv6::from_u128(self.addr.to_u128() & mask)
    }

    /// Broadcast (last address) for this CIDR.
    pub fn broadcast(self) -> Ipv6 {
        let mask = cidr_mask_v6(self.prefix);
        Ipv6::from_u128(self.addr.to_u128() | !mask)
    }

    /// Whether the CIDR contains the given address.
    pub fn contains(self, ip: Ipv6) -> bool {
        let mask = cidr_mask_v6(self.prefix);
        (self.addr.to_u128() & mask) == (ip.to_u128() & mask)
    }
}

impl fmt::Display for Cidr6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.prefix)
    }
}

/// An IP CIDR that may be IPv4 or IPv6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cidr {
    V4(Cidr4),
    V6(Cidr6),
}

impl Cidr {
    /// Parse a string such as `10.0.0.0/8` or `2001:db8::/32`.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        if s.contains(':') {
            Ok(Cidr::V6(Cidr6::parse(s)?))
        } else {
            Ok(Cidr::V4(Cidr4::parse(s)?))
        }
    }

    /// Network address for this CIDR.
    pub fn network(self) -> IpAddr {
        match self {
            Cidr::V4(c) => IpAddr::V4(c.network()),
            Cidr::V6(c) => IpAddr::V6(c.network()),
        }
    }

    /// Last address for this CIDR.
    pub fn broadcast(self) -> IpAddr {
        match self {
            Cidr::V4(c) => IpAddr::V4(c.broadcast()),
            Cidr::V6(c) => IpAddr::V6(c.broadcast()),
        }
    }

    /// Whether this CIDR contains the given address.
    pub fn contains(self, ip: &IpAddr) -> bool {
        match (self, ip) {
            (Cidr::V4(c), IpAddr::V4(a)) => c.contains(*a),
            (Cidr::V6(c), IpAddr::V6(a)) => c.contains(*a),
            _ => false,
        }
    }

    /// The prefix length.
    pub fn prefix(self) -> u8 {
        match self {
            Cidr::V4(c) => c.prefix,
            Cidr::V6(c) => c.prefix,
        }
    }
}

impl fmt::Display for Cidr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cidr::V4(c) => c.fmt(f),
            Cidr::V6(c) => c.fmt(f),
        }
    }
}

fn cidr_mask_v4(prefix: u8) -> u32 {
    match prefix {
        0 => 0,
        32 => u32::MAX,
        _ => u32::MAX << (32 - prefix as u32),
    }
}

fn cidr_mask_v6(prefix: u8) -> u128 {
    match prefix {
        0 => 0,
        128 => u128::MAX,
        _ => u128::MAX << (128 - prefix as u32),
    }
}
