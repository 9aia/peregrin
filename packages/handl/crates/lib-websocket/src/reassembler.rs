use crate::{Error, Frame, Message, OpCode};

/// Reassembles fragmented WebSocket frames into complete [`Message`]s.
///
/// Control frames are yielded immediately and can interleave with fragmented
/// data frames.
#[derive(Debug, Clone, Default)]
pub struct Reassembler {
    pub(crate) buffer: Vec<u8>,
    pub(crate) start_opcode: Option<OpCode>,
}

impl Reassembler {
    /// Create a new, empty reassembler.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            start_opcode: None,
        }
    }

    /// Feed a single frame and return any complete message.
    ///
    /// Returns `Ok(None)` when the frame is a data fragment that has not yet
    /// completed a message.
    pub fn feed(&mut self, frame: &Frame) -> Result<Option<Message>, Error> {
        if frame.opcode.is_control() {
            if !frame.fin || frame.payload.len() > 125 {
                return Err(Error::FragmentedControl);
            }
            return Message::from_frame(frame).map(Some);
        }

        match (self.start_opcode, frame.opcode) {
            (None, OpCode::Continuation) => Err(Error::UnexpectedContinuation),
            (Some(_), OpCode::Text | OpCode::Binary) => Err(Error::UnexpectedContinuation),
            (None, OpCode::Text | OpCode::Binary) => {
                self.start_opcode = Some(frame.opcode);
                if frame.fin {
                    let msg = Message::from_frame(frame)?;
                    self.reset();
                    return Ok(Some(msg));
                }
                self.buffer.extend_from_slice(&frame.payload);
                Ok(None)
            }
            (Some(start), OpCode::Continuation) => {
                self.buffer.extend_from_slice(&frame.payload);
                if frame.fin {
                    let payload = std::mem::take(&mut self.buffer);
                    let _ = start;
                    let opcode = self.start_opcode.take().unwrap();
                    let assembled = Frame {
                        fin: true,
                        rsv1: false,
                        rsv2: false,
                        rsv3: false,
                        opcode,
                        mask: None,
                        payload,
                    };
                    let msg = Message::from_frame(&assembled)?;
                    return Ok(Some(msg));
                }
                Ok(None)
            }
            _ => Err(Error::UnexpectedContinuation),
        }
    }

    /// Reset the reassembler to its initial state.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.start_opcode = None;
    }
}
