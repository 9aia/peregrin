use core::fmt;

/// Errors that can occur when parsing or operating on IP values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpError {
    InvalidAddress,
    InvalidCidr,
    InvalidPort,
}

impl fmt::Display for IpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpError::InvalidAddress => f.write_str("invalid IP address"),
            IpError::InvalidCidr => f.write_str("invalid CIDR notation"),
            IpError::InvalidPort => f.write_str("invalid port number"),
        }
    }
}

impl core::error::Error for IpError {}
