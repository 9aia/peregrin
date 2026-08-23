mod socket;

pub use socket::{Socket, SocketModule};

pub mod socket_stub;

#[cfg(test)]
mod socket_tests;
