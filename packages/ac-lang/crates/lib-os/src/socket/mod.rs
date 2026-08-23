mod socket;
mod async_socket;

pub use socket::{Socket, SocketModule};
pub use async_socket::{AsyncSocket, AsyncSocketModule};

pub mod socket_stub;

#[cfg(test)]
mod socket_tests;
