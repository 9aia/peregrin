use crate::Error;

/// WebSocket opcode as defined by RFC 6455.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpCode {
    /// Continuation frame.
    Continuation,
    /// Text frame.
    Text,
    /// Binary frame.
    Binary,
    /// Close frame.
    Close,
    /// Ping frame.
    Ping,
    /// Pong frame.
    Pong,
}

impl OpCode {
    /// Convert an opcode byte into an `OpCode`.
    pub fn from_u8(value: u8) -> Result<Self, Error> {
        match value {
            0x0 => Ok(Self::Continuation),
            0x1 => Ok(Self::Text),
            0x2 => Ok(Self::Binary),
            0x8 => Ok(Self::Close),
            0x9 => Ok(Self::Ping),
            0xA => Ok(Self::Pong),
            other => Err(Error::InvalidOpCode(other)),
        }
    }

    /// Return the wire representation of this opcode.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Continuation => 0x0,
            Self::Text => 0x1,
            Self::Binary => 0x2,
            Self::Close => 0x8,
            Self::Ping => 0x9,
            Self::Pong => 0xA,
        }
    }

    /// `true` for control opcodes (`Close`, `Ping`, `Pong`).
    pub const fn is_control(self) -> bool {
        matches!(self, Self::Close | Self::Ping | Self::Pong)
    }

    /// `true` for data opcodes (`Text`, `Binary`, `Continuation`).
    pub const fn is_data(self) -> bool {
        !self.is_control()
    }
}
