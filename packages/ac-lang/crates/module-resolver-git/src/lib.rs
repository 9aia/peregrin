//! # ac-module-resolver-git
//!
//! Git-based [`ModuleResolver`] implementation.
//!
//! Fetches documents from Git repositories using `git2`. Module paths use
//! the `git+` URL scheme:
//!
//! ```text
//! import('git+https://github.com/org/repo.git@main/schemas/base.yml')
//! ```
//!
//! Repositories are cloned to a cache directory keyed by a SHA-256 hash
//! of `(repo_url, ref)`. Clones are **bare** and **shallow** (depth=1 by
//! default) to minimize download size. Subsequent resolves from the same
//! repo+ref fetch updates from the remote before reading.

mod parse;
mod fetch;
mod git_resolver;

pub use parse::{parse_git_url, GitTarget};
pub use fetch::{GitFetcher, DEFAULT_DEPTH};
pub use git_resolver::GitResolver;

#[cfg(test)]
mod parse_tests;
#[cfg(test)]
mod fetch_tests;
#[cfg(test)]
mod lib_tests;
