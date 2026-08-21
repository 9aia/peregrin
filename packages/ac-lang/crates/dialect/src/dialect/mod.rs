mod dialect;

pub use dialect::Dialect;

#[cfg(test)]
pub(crate) mod dialect_stub;

#[cfg(test)]
mod dialect_tests;
