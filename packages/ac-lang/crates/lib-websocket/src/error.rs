use std::{error, fmt, io};

/// Errors that can occur when encoding or decoding WebSocket frames.
#[derive(Debug)]
pub enum Error {
    /// Underlying I/O error.
    Io(io::Error),
    /// An unknown opcode was encountered.
    InvalidOpCode(u8),
    /// A payload length field is malformed or reserved.
    InvalidPayloadLength,
    /// Not enough bytes were available to complete the operation.
    Incomplete,
    /// A control frame was fragmented or exceeded the maximum control payload.
    FragmentedControl,
    /// Text payload is not valid UTF-8.
    InvalidUtf8,
    /// A close frame payload is malformed.
    InvalidCloseFrame,
    /// A continuation frame arrived without a starting text/binary frame.
    UnexpectedContinuation,
    /// One or more reserved bits were set and are not supported.
    ReservedBits,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::InvalidOpCode(b) => write!(f, "invalid websocket opcode: {b}"),
            Error::InvalidPayloadLength => write!(f, "invalid payload length"),
            Error::Incomplete => write!(f, "incomplete websocket frame"),
            Error::FragmentedControl => write!(f, "fragmented or oversized control frame"),
            Error::InvalidUtf8 => write!(f, "invalid utf-8 in text frame"),
            Error::InvalidCloseFrame => write!(f, "invalid close frame payload"),
            Error::UnexpectedContinuation => write!(f, "unexpected continuation frame"),
            Error::ReservedBits => write!(f, "reserved bits are not supported"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
