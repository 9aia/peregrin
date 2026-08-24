use crate::{CloseFrame, Error, Frame, OpCode};

/// A complete WebSocket message.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// A text message.
    Text(String),
    /// A binary message.
    Binary(Vec<u8>),
    /// A ping message.
    Ping(Vec<u8>),
    /// A pong message.
    Pong(Vec<u8>),
    /// A close message with an optional close frame.
    Close(Option<CloseFrame>),
}

impl Message {
    /// Convert a single, unfragmented frame into a message.
    pub fn from_frame(frame: &Frame) -> Result<Self, Error> {
        if !frame.fin {
            return Err(Error::Incomplete);
        }
        match frame.opcode {
            OpCode::Text => {
                let text = std::str::from_utf8(&frame.payload)
                    .map_err(|_| Error::InvalidUtf8)?
                    .to_string();
                Ok(Self::Text(text))
            }
            OpCode::Binary => Ok(Self::Binary(frame.payload.clone())),
            OpCode::Ping => Ok(Self::Ping(frame.payload.clone())),
            OpCode::Pong => Ok(Self::Pong(frame.payload.clone())),
            OpCode::Close => {
                if frame.payload.is_empty() {
                    Ok(Self::Close(None))
                } else {
                    Ok(Self::Close(Some(CloseFrame::decode(&frame.payload)?)))
                }
            }
            OpCode::Continuation => Err(Error::UnexpectedContinuation),
        }
    }

    /// Convert this message into a single frame.
    ///
    /// `mask` should be provided by clients per RFC 6455. Servers omit it.
    pub fn into_frame(self, mask: Option<[u8; 4]>) -> Frame {
        match self {
            Self::Text(text) => Frame::new(OpCode::Text, text.into_bytes()).with_mask_opt(mask),
            Self::Binary(bytes) => Frame::new(OpCode::Binary, bytes).with_mask_opt(mask),
            Self::Ping(data) => Frame::new(OpCode::Ping, data).with_mask_opt(mask),
            Self::Pong(data) => Frame::new(OpCode::Pong, data).with_mask_opt(mask),
            Self::Close(frame) => {
                let payload = frame.as_ref().map_or_else(Vec::new, |f| f.encode());
                Frame::new(OpCode::Close, payload).with_mask_opt(mask)
            }
        }
    }
}
