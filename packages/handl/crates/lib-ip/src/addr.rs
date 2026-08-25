use crate::IpError;
use core::fmt;

/// An IPv4 address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ipv4(u32);

impl Ipv4 {
    /// Construct from four octets.
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Ipv4(u32::from_be_bytes([a, b, c, d]))
    }

    /// Parse a dotted-decimal address.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        let mut parts = s.split('.');
        let mut octets = [0u8; 4];
        for octet in &mut octets {
            match parts.next() {
                Some(part) if !part.is_empty() => {
                    if part.len() > 1 && part.starts_with('0') {
                        return Err(IpError::InvalidAddress);
                    }
                    *octet = part.parse().map_err(|_| IpError::InvalidAddress)?;
                }
                _ => return Err(IpError::InvalidAddress),
            }
        }
        if parts.next().is_some() {
            return Err(IpError::InvalidAddress);
        }
        Ok(Ipv4(u32::from_be_bytes(octets)))
    }

    /// Return the four octets.
    pub const fn octets(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    /// Pack the address into a `u32` in network byte order.
    pub const fn to_u32(self) -> u32 {
        self.0
    }

    /// Construct from a `u32` in network byte order.
    pub const fn from_u32(value: u32) -> Self {
        Ipv4(value)
    }

    /// `127.0.0.0/8`.
    pub fn is_loopback(self) -> bool {
        self.octets()[0] == 127
    }

    /// `10.0.0.0/8`, `172.16.0.0/12`, or `192.168.0.0/16`.
    pub fn is_private(self) -> bool {
        let [a, b, _, _] = self.octets();
        a == 10 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168)
    }

    /// `169.254.0.0/16`.
    pub fn is_link_local(self) -> bool {
        let [a, b, _, _] = self.octets();
        a == 169 && b == 254
    }

    /// `224.0.0.0/4`.
    pub fn is_multicast(self) -> bool {
        let [a, _, _, _] = self.octets();
        (224..=239).contains(&a)
    }

    /// `0.0.0.0`.
    pub fn is_unspecified(self) -> bool {
        self.0 == 0
    }

    /// `255.255.255.255`.
    pub fn is_broadcast(self) -> bool {
        self.0 == u32::MAX
    }
}

impl fmt::Display for Ipv4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d] = self.octets();
        write!(f, "{}.{}.{}.{}", a, b, c, d)
    }
}

/// An IPv6 address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ipv6(u128);

impl Ipv6 {
    /// Construct from eight 16-bit segments.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        Self::from_segments([a, b, c, d, e, f, g, h])
    }

    /// Parse an IPv6 address in the usual textual notation.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        if s.is_empty() || !s.contains(':') {
            return Err(IpError::InvalidAddress);
        }
        let expanded = if s.contains("::") {
            expand_ipv6(s)?
        } else {
            s.to_string()
        };
        let mut segments = [0u16; 8];
        let parts: Vec<&str> = expanded.split(':').collect();
        if parts.len() != 8 {
            return Err(IpError::InvalidAddress);
        }
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() || part.len() > 4 {
                return Err(IpError::InvalidAddress);
            }
            segments[i] = u16::from_str_radix(part, 16).map_err(|_| IpError::InvalidAddress)?;
        }
        Ok(Ipv6::from_segments(segments))
    }

    /// Construct from eight 16-bit segments.
    pub const fn from_segments(segments: [u16; 8]) -> Self {
        let mut value: u128 = 0;
        let mut i = 0;
        while i < 8 {
            value = (value << 16) | (segments[i] as u128);
            i += 1;
        }
        Ipv6(value)
    }

    /// Return the eight 16-bit segments.
    pub const fn segments(self) -> [u16; 8] {
        let bytes = self.0.to_be_bytes();
        [
            u16::from_be_bytes([bytes[0], bytes[1]]),
            u16::from_be_bytes([bytes[2], bytes[3]]),
            u16::from_be_bytes([bytes[4], bytes[5]]),
            u16::from_be_bytes([bytes[6], bytes[7]]),
            u16::from_be_bytes([bytes[8], bytes[9]]),
            u16::from_be_bytes([bytes[10], bytes[11]]),
            u16::from_be_bytes([bytes[12], bytes[13]]),
            u16::from_be_bytes([bytes[14], bytes[15]]),
        ]
    }

    /// Pack the address into a `u128` in network byte order.
    pub const fn to_u128(self) -> u128 {
        self.0
    }

    /// Construct from a `u128` in network byte order.
    pub const fn from_u128(value: u128) -> Self {
        Ipv6(value)
    }

    /// `::1`.
    pub fn is_loopback(self) -> bool {
        self.0 == 1
    }

    /// `ff00::/8`.
    pub fn is_multicast(self) -> bool {
        let segments = self.segments();
        (segments[0] & 0xff00) == 0xff00
    }

    /// `::`.
    pub fn is_unspecified(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Ipv6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let segments = self.segments();
        let (start, len) = longest_zero_run(&segments);
        let mut out = String::new();
        if len >= 2 {
            for seg in &segments[..start] {
                if !out.is_empty() {
                    out.push(':');
                }
                out.push_str(&format!("{:x}", seg));
            }
            out.push_str("::");
            for seg in &segments[start + len..] {
                if !out.is_empty() && !out.ends_with(':') {
                    out.push(':');
                }
                out.push_str(&format!("{:x}", seg));
            }
        } else {
            for (i, seg) in segments.iter().enumerate() {
                if i > 0 {
                    out.push(':');
                }
                out.push_str(&format!("{:x}", seg));
            }
        }
        f.write_str(&out)
    }
}

/// An IP address that may be IPv4 or IPv6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IpAddr {
    V4(Ipv4),
    V6(Ipv6),
}

impl IpAddr {
    /// Parse an IPv4 or IPv6 address string.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        if s.contains(':') {
            Ok(IpAddr::V6(Ipv6::parse(s)?))
        } else {
            Ok(IpAddr::V4(Ipv4::parse(s)?))
        }
    }

    /// `127.0.0.1` or `::1`.
    pub fn is_loopback(self) -> bool {
        match self {
            IpAddr::V4(a) => a.is_loopback(),
            IpAddr::V6(a) => a.is_loopback(),
        }
    }

    /// Whether the address is in a multicast range.
    pub fn is_multicast(self) -> bool {
        match self {
            IpAddr::V4(a) => a.is_multicast(),
            IpAddr::V6(a) => a.is_multicast(),
        }
    }

    /// `0.0.0.0` or `::`.
    pub fn is_unspecified(self) -> bool {
        match self {
            IpAddr::V4(a) => a.is_unspecified(),
            IpAddr::V6(a) => a.is_unspecified(),
        }
    }

    /// Whether the address is in a private IPv4 range.
    pub fn is_private(self) -> bool {
        match self {
            IpAddr::V4(a) => a.is_private(),
            IpAddr::V6(_) => false,
        }
    }

    /// Whether the address is an IPv4 link-local address.
    pub fn is_link_local(self) -> bool {
        match self {
            IpAddr::V4(a) => a.is_link_local(),
            IpAddr::V6(_) => false,
        }
    }
}

impl fmt::Display for IpAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpAddr::V4(a) => a.fmt(f),
            IpAddr::V6(a) => a.fmt(f),
        }
    }
}

fn expand_ipv6(s: &str) -> Result<String, IpError> {
    if s.matches("::").count() != 1 {
        return Err(IpError::InvalidAddress);
    }
    let mut parts = s.splitn(2, "::");
    let before = parts.next().unwrap_or("");
    let after = parts.next().unwrap_or("");

    let before_parts: Vec<&str> = if before.is_empty() {
        Vec::new()
    } else {
        before.split(':').collect()
    };
    let after_parts: Vec<&str> = if after.is_empty() {
        Vec::new()
    } else {
        after.split(':').collect()
    };

    let missing = 8usize.saturating_sub(before_parts.len() + after_parts.len());
    if missing == 0 {
        return Err(IpError::InvalidAddress);
    }

    let mut all: Vec<&str> = Vec::with_capacity(8);
    all.extend_from_slice(&before_parts);
    all.extend(std::iter::repeat_n("0", missing));
    all.extend_from_slice(&after_parts);

    if all.len() != 8 {
        return Err(IpError::InvalidAddress);
    }
    Ok(all.join(":"))
}

fn longest_zero_run(segments: &[u16; 8]) -> (usize, usize) {
    let mut best_start = 8;
    let mut best_len = 0;
    let mut cur_start = 0;
    let mut cur_len = 0;

    for (i, &s) in segments.iter().enumerate() {
        if s == 0 {
            if cur_len == 0 {
                cur_start = i;
            }
            cur_len += 1;
            if cur_len > best_len {
                best_start = cur_start;
                best_len = cur_len;
            }
        } else {
            cur_len = 0;
        }
    }

    (best_start, best_len)
}
