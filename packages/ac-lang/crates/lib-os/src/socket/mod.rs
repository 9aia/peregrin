mod async_socket;
mod socket;

pub use async_socket::{AsyncSocket, AsyncSocketModule};
pub use socket::{Socket, SocketModule};

pub mod socket_stub;

#[cfg(test)]
mod socket_tests;
