use crate::{IpAddr, IpError, Ipv6};
use core::fmt;

/// A socket address (IP + port).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SocketAddr {
    pub ip: IpAddr,
    pub port: u16,
}

impl SocketAddr {
    /// Parse a string such as `127.0.0.1:8080` or `[::1]:443`.
    pub fn parse(s: &str) -> Result<Self, IpError> {
        if let Some(body) = s.strip_prefix('[') {
            let Some(end) = body.find(']') else {
                return Err(IpError::InvalidAddress);
            };
            let ip_str = &body[..end];
            let rest = &body[end..];
            if !rest.starts_with("]:") || rest.len() == 2 {
                return Err(IpError::InvalidPort);
            }
            let port_str = &rest[2..];
            let port = port_str.parse::<u16>().map_err(|_| IpError::InvalidPort)?;
            let ip = if ip_str.contains(':') {
                IpAddr::V6(Ipv6::parse(ip_str)?)
            } else {
                IpAddr::parse(ip_str)?
            };
            Ok(SocketAddr { ip, port })
        } else {
            let Some((ip_str, port_str)) = s.rsplit_once(':') else {
                return Err(IpError::InvalidPort);
            };
            if port_str.is_empty() {
                return Err(IpError::InvalidPort);
            }
            let port = port_str.parse::<u16>().map_err(|_| IpError::InvalidPort)?;
            let ip = IpAddr::parse(ip_str)?;
            Ok(SocketAddr { ip, port })
        }
    }

    /// The IP address.
    pub fn ip(self) -> IpAddr {
        self.ip
    }

    /// The port number.
    pub fn port(self) -> u16 {
        self.port
    }

    /// Whether the IP address is a loopback address.
    pub fn is_loopback(self) -> bool {
        self.ip.is_loopback()
    }
}

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ip {
            IpAddr::V4(a) => write!(f, "{}:{}", a, self.port),
            IpAddr::V6(a) => write!(f, "[{}]:{}", a, self.port),
        }
    }
}
