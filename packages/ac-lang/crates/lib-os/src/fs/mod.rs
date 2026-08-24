mod fs;

pub use fs::{Fs, FsModule};

pub mod fs_stub;

#[cfg(test)]
mod fs_tests;