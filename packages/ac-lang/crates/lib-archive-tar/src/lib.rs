//! # ac-lib-archive-tar
//!
//! Tar implementation of the [`ac_lib_archive`] [`Archive`] trait using [`tar`].

mod tar_archive;

#[cfg(test)]
mod tar_tests;

pub use tar_archive::TarArchive;
