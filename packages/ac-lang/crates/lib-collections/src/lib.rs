//! # ac-lib-collections
//!
//! Collection types for the AC runtime.
//!
//! Provides modules for common collection categories:
//!
//! - `maps` — key-value maps
//! - `sets` — unordered sets
//! - `vectors` — growable arrays
//! - `queues` — first-in-first-out and double-ended queues
//! - `trees` — ordered binary search trees

pub mod maps;
pub mod queues;
pub mod sets;
pub mod trees;
pub mod vectors;

#[cfg(test)]
mod maps_tests;
#[cfg(test)]
mod queues_tests;
#[cfg(test)]
mod sets_tests;
#[cfg(test)]
mod trees_tests;
#[cfg(test)]
mod vectors_tests;
