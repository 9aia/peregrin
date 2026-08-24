#![forbid(unsafe_code)]
#![allow(missing_docs)]

//! WebSocket frame and message codec.
//!
//! This crate provides a fast, dependency-free implementation of the WebSocket
//! framing protocol (RFC 6455). It is designed to be embedded in clients,
//! servers, and runtime modules that need to produce or consume WebSocket
//! frames without committing to a particular async runtime or TLS stack.
//!
//! # Example
//!
//! ```
//! use ac_lib_websocket::{Frame, Message, OpCode};
//!
//! let frame = Frame::new(OpCode::Text, b"hello".to_vec());
//! let bytes = frame.encode();
//! let (decoded, _) = Frame::decode(&bytes).unwrap();
//! let message = Message::from_frame(&decoded).unwrap();
//! assert_eq!(message, Message::Text("hello".to_string()));
//! ```

pub mod close;
pub mod error;
pub mod frame;
pub mod mask;
pub mod message;
pub mod opcode;
pub mod reassembler;

pub use close::{CloseCode, CloseFrame};
pub use error::Error;
pub use frame::Frame;
pub use message::Message;
pub use opcode::OpCode;
pub use reassembler::Reassembler;

#[cfg(test)]
mod tests;
