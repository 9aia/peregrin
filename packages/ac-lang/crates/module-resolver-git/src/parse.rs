//! Parse `git+` URLs into components.
//!
//! Format: `git+<scheme>://<host>/<org>/<repo>.git@<ref>/<path/to/file>`
//!
//! Example:
//! ```text
//! git+https://github.com/org/repo.git@main/schemas/base.yml
//! ```
//! → repo_url: `https://github.com/org/repo.git`
//!   ref: `main`
//!   path: `schemas/base.yml`

use url::Url;

/// Parsed components of a `git+` URL.
pub struct GitTarget {
    /// Repository URL without the `git+` prefix and without the `@ref/path` suffix.
    pub repo_url: String,
    /// Git reference (branch, tag, or commit hash).
    pub reference: String,
    /// Path to the file within the repository.
    pub path: String}

/// Parse a `git+` URL into [`GitTarget`].
///
/// Returns `None` if the URL is not a valid `git+` URL.
pub fn parse_git_url(input: &str) -> Option<GitTarget> {
    let rest = input.strip_prefix("git+")?;
    let url = Url::parse(rest).ok()?;

    let full_path = url.path();
    let (repo_path, after_git) = full_path.split_once(".git")?;
    let after_git = after_git
        .strip_prefix('@')
        .or_else(|| after_git.strip_prefix('/'))
        .unwrap_or(after_git);

    let (reference, file_path) = after_git
        .split_once('/')
        .map(|(r, p)| (r.to_string(), p.to_string()))
        .unwrap_or((after_git.to_string(), String::new()));

    let repo_url = format!("{}://{}{}.git", url.scheme(), url.host_str()?, repo_path);

    Some(GitTarget {
        repo_url,
        reference,
        path: file_path})
}
