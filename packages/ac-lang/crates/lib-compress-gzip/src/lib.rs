//! # ac-lib-compress-gzip
//!
//! Gzip implementation of the [`ac_lib_compress`] traits using [`flate2`].

mod gzip;

#[cfg(test)]
mod gzip_tests;

pub use gzip::Gzip;
