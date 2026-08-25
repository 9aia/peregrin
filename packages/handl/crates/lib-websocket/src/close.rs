use std::fmt;

use crate::Error;

/// A WebSocket close code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CloseCode(pub u16);

impl CloseCode {
    /// Normal closure.
    pub const NORMAL: Self = Self(1000);
    /// Endpoint going away.
    pub const GOING_AWAY: Self = Self(1001);
    /// Protocol error.
    pub const PROTOCOL_ERROR: Self = Self(1002);
    /// Unsupported data.
    pub const UNSUPPORTED_DATA: Self = Self(1003);
    /// No status received.
    pub const NO_STATUS_RECEIVED: Self = Self(1005);
    /// Abnormal closure.
    pub const ABNORMAL: Self = Self(1006);
    /// Invalid frame payload data.
    pub const INVALID_FRAME_PAYLOAD: Self = Self(1007);
    /// Policy violation.
    pub const POLICY_VIOLATION: Self = Self(1008);
    /// Message too big.
    pub const MESSAGE_TOO_BIG: Self = Self(1009);
    /// Mandatory extension missing.
    pub const MANDATORY_EXTENSION: Self = Self(1010);
    /// Internal server error.
    pub const INTERNAL_SERVER_ERROR: Self = Self(1011);
    /// TLS handshake failure.
    pub const TLS_HANDSHAKE: Self = Self(1015);

    /// Convert two big-endian bytes into a close code.
    pub fn from_be_bytes(bytes: [u8; 2]) -> Self {
        Self(u16::from_be_bytes(bytes))
    }

    /// Encode this close code as two big-endian bytes.
    pub fn to_be_bytes(self) -> [u8; 2] {
        self.0.to_be_bytes()
    }
}

impl From<u16> for CloseCode {
    fn from(code: u16) -> Self {
        Self(code)
    }
}

impl fmt::Display for CloseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A parsed WebSocket close frame payload.
#[derive(Debug, Clone, PartialEq)]
pub struct CloseFrame {
    /// Status code.
    pub code: CloseCode,
    /// Optional human-readable reason.
    pub reason: String,
}

impl CloseFrame {
    /// Create a new close frame.
    pub fn new(code: impl Into<CloseCode>, reason: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            reason: reason.into(),
        }
    }

    /// Encode a close frame payload: 2-byte code followed by UTF-8 reason.
    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(2 + self.reason.len());
        v.extend_from_slice(&self.code.to_be_bytes());
        v.extend_from_slice(self.reason.as_bytes());
        v
    }

    /// Decode a close frame payload.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 2 {
            return Err(Error::InvalidCloseFrame);
        }
        let code = CloseCode::from_be_bytes([bytes[0], bytes[1]]);
        let reason = if bytes.len() > 2 {
            std::str::from_utf8(&bytes[2..])
                .map_err(|_| Error::InvalidUtf8)?
                .to_string()
        } else {
            String::new()
        };
        Ok(Self { code, reason })
    }
}
