//! peregrin-source — the bundle-source port (ADR-0079). A bundle is
//! bytes + manifest whether it lives on disk, in a git repo, or behind
//! HTTP, so the *ref* grammar lives here in the port and every impl
//! shares it:
//!
//! ```text
//! ./path  ../path  /abs/path  ~/path  nested/path   → source-fs
//! git:<url>[//<subdir>][#<ref>]                     → source-git
//! https://…/name.tgz | .tar.gz [#sha256:<hex>]      → source-http
//! ```
//!
//! Git specifics (the go-getter conventions the ADR adopts): `//`
//! splits the repo path from the in-repo path — required because
//! nested groups make bare `a/b/c/d` ambiguous. Without `//`, a
//! two-segment prefix is the repo and the remainder is the in-repo
//! path — but only for *shorthand* paths; anything containing `://`
//! (or scp-style `host:path`, which splits on `:`) is already a full
//! clone URL and parses whole. `org/repo` defaults to github.com;
//! `host/org/repo` becomes `https://host/org/repo`; `git@host:…` and
//! `ssh://`/`file://` URLs pass through. `#ref` is branch/tag/sha —
//! the version pin; the resolved commit sha is recorded regardless.
//!
//! Impls (`peregrin-source-{fs,git,http}`) provide [`Source`]:
//! [`Source::resolve`] turns a ref into a [`Resolved`] fetch spec
//! (git pins `#rev` to a commit sha here, via `ls-remote`), and
//! [`Source::fetch`] materializes the payload into a caller-owned
//! dir. The fetch→unpack split is what makes the content-addressed
//! cache work — repeat installs and air-gapped nodes reuse
//! `~/.cache/peregrin/fetch/<key>` entries instead of re-fetching.

use std::io;
use std::path::{Path, PathBuf};

/// A parsed bundle ref — which [`Source`] impl handles it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceRef {
    /// A local path — `./x`, `/x`, `~/x`, `nested/x`.
    Fs(PathBuf),
    /// `git:` — any clone URL with optional `//subdir` and `#rev`.
    Git(GitRef),
    /// `http(s)://….tgz|tar.gz` with an optional `#sha256:` pin.
    Http(HttpRef),
}

/// The `git:` arm of a [`SourceRef`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRef {
    /// Normalized clone URL (`org/repo` → `https://github.com/org/repo`).
    pub url: String,
    /// In-repo path after `//` (or the post-two-segment remainder).
    pub subdir: Option<String>,
    /// `#<branch|tag|sha>` — the version pin.
    pub rev: Option<String>,
}

/// The tarball arm of a [`SourceRef`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRef {
    /// URL minus the `#sha256:` fragment.
    pub url: String,
    /// Expected sha256 hex of the tarball bytes, when pinned.
    pub digest: Option<String>,
}

/// What [`Source::resolve`] produces — the concrete fetch spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// Local dir or tarball file to copy/unpack.
    Fs {
        /// `~`-expanded path.
        path: PathBuf,
    },
    /// A git clone — `sha` is filled when resolve pinned it via
    /// `ls-remote` (or the `#rev` was already a full sha).
    Git {
        /// Normalized clone URL.
        url: String,
        /// In-repo path, if any.
        subdir: Option<String>,
        /// The requested ref (kept for fetch when `sha` is None).
        rev: Option<String>,
        /// Resolved commit sha — the pin recorded in the store.
        sha: Option<String>,
    },
    /// A tarball GET.
    Http {
        /// Fetch URL.
        url: String,
        /// Expected tarball sha256, when the ref pinned it.
        digest: Option<String>,
    },
}

impl Resolved {
    /// `(content_key, resolved_id)` — both knowable *before* the fetch
    /// or neither: `git-<sha>` + the sha for resolved/pinned git refs,
    /// `sha256-<hex>` + `sha256:<hex>` for digest-pinned tarballs.
    /// `None` means the caller digests the fetched payload to learn
    /// the key — the [`Fetched`] then carries both.
    #[must_use]
    pub fn prefetch_identity(&self) -> Option<(String, String)> {
        match self {
            Self::Git { sha: Some(sha), .. } => Some((format!("git-{sha}"), sha.clone())),
            Self::Http {
                digest: Some(digest),
                ..
            } => Some((format!("sha256-{digest}"), format!("sha256:{digest}"))),
            _ => None,
        }
    }
}

/// What [`Source::fetch`] produced — an unpacked payload plus the
/// identity the store records.
#[derive(Clone, Debug)]
pub struct Fetched {
    /// Dir holding the unpacked payload.
    pub dir: PathBuf,
    /// Resolved identifier recorded in the store manifest — the git
    /// commit sha or `sha256:<hex>` content digest.
    pub resolved: String,
    /// Content-addressed cache key (`git-<sha>` / `sha256-<hex>`).
    pub content_key: String,
}

/// The bundle-source port. One impl per transport; `resolve` +
/// `fetch` are split so the caller can hit the content-addressed
/// cache between them.
pub trait Source: Send + Sync {
    /// Turn a parsed ref into a fetch spec — may hit the transport
    /// (git resolves `#rev` via `ls-remote` here).
    fn resolve(&self, reference: &SourceRef) -> Result<Resolved, Error>;

    /// Materialize the payload into `dest_dir` (created as needed)
    /// and report what it resolved to.
    fn fetch(&self, resolved: &Resolved, dest_dir: &Path) -> Result<Fetched, Error>;
}

/// Source/grammar failures.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The ref didn't parse against the ADR-0079 grammar.
    #[error("invalid ref: {0}")]
    Ref(String),
    /// A filesystem operation failed.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },
    /// Transport or payload validation failure.
    #[error("{0}")]
    Failed(String),
}

impl Error {
    /// Shorthand for [`Error::Io`].
    pub fn io(context: impl Into<String>) -> impl FnOnce(io::Error) -> Self {
        move |source| Self::Io {
            context: context.into(),
            source,
        }
    }
}

/// `~` / `~/…` expansion, matching the workspace `home_dir`
/// convention (`$HOME`, else `$USERPROFILE`, else `/`). Non-`~` paths
/// and non-UTF-8 paths pass through untouched — same semantics as
/// `peregrin_git::expand_path`, duplicated so the port stays impl-free.
#[must_use]
pub fn expand_path(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    if text == "~" {
        return home_dir();
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    path.to_path_buf()
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// Parse an ADR-0079 bundle ref into its [`SourceRef`] arm.
pub fn parse_ref(input: &str) -> Result<SourceRef, Error> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Error::Ref("empty ref".to_string()));
    }
    if let Some(rest) = input.strip_prefix("git:") {
        return parse_git(rest).map(SourceRef::Git);
    }
    if input.starts_with("https://") || input.starts_with("http://") {
        return parse_http(input).map(SourceRef::Http);
    }
    if is_path_ref(input) {
        return Ok(SourceRef::Fs(PathBuf::from(input)));
    }
    Err(Error::Ref(format!(
        "unrecognized ref '{input}' — expected ./path, /path, git:<url>[//subdir][#rev], or an http(s) .tgz/.tar.gz URL"
    )))
}

/// Path-shaped refs — `./x`, `../x`, `~/x`, `/x`, plus bare paths
/// carrying a separator (`libraries/ci-gate`). A bare name never is:
/// `peregrin install ci-gate` errors rather than guessing.
fn is_path_ref(input: &str) -> bool {
    input == "."
        || input == ".."
        || input == "~"
        || input.starts_with("./")
        || input.starts_with("../")
        || input.starts_with("~/")
        || input.starts_with('/')
        || input.contains('/')
}

/// `git:<rest>` — `#rev` off the tail, `//subdir` out of the middle,
/// then clone-URL normalization.
fn parse_git(rest: &str) -> Result<GitRef, Error> {
    if rest.is_empty() {
        return Err(Error::Ref("git: ref is empty".to_string()));
    }
    // '#rev' — '#' is legal in neither clone URLs nor git refs, so the
    // first one splits and a second one is a hard error.
    let (head, rev) = match rest.split_once('#') {
        Some((head, rev)) => {
            if rev.is_empty() {
                return Err(Error::Ref("empty ref after '#'".to_string()));
            }
            if rev.contains('#') {
                return Err(Error::Ref(format!("malformed ref '#{rev}'")));
            }
            (head, Some(rev.to_string()))
        }
        None => (rest, None),
    };
    if head.is_empty() {
        return Err(Error::Ref("git: ref missing the repo".to_string()));
    }
    let (repo, subdir) = split_subdir(head)?;
    let url = normalize_clone_url(repo);
    Ok(GitRef { url, subdir, rev })
}

/// Pull the `//subdir` out of `head`; fall back to the two-segment
/// rule for shorthand paths.
fn split_subdir(head: &str) -> Result<(&str, Option<String>), Error> {
    // `//` inside `scheme://` doesn't count — start scanning past it.
    let scan = head.find("://").map_or(0, |i| i + 3);
    if let Some(i) = head[scan..].find("//") {
        let repo = &head[..scan + i];
        let subdir = &head[scan + i + 2..];
        let subdir = subdir.trim_end_matches('/');
        if subdir.is_empty() || subdir == "." {
            return Ok((repo, None));
        }
        if subdir.starts_with('/') || subdir.split('/').any(|c| c == "..") {
            return Err(Error::Ref(format!("bad in-repo path '//{subdir}'")));
        }
        return Ok((repo, Some(subdir.to_string())));
    }
    // No //: scheme URLs, scp-style `host:path`, and explicit local
    // paths (`/x`, `~/x`, `./x`, `../x`) are whole repos; only
    // shorthand paths split after the second segment.
    if head.contains("://")
        || is_scp_style(head)
        || head.starts_with('/')
        || head.starts_with('.')
        || head.starts_with('~')
    {
        return Ok((head, None));
    }
    // Two-segment prefix = repo, remainder = in-repo path.
    let Some(first_slash) = head.find('/') else {
        return Ok((head, None));
    };
    let Some(second_slash) = head[first_slash + 1..].find('/') else {
        return Ok((head, None));
    };
    let repo_end = first_slash + 1 + second_slash;
    let subdir = head[repo_end + 1..].trim_end_matches('/');
    if subdir.is_empty() || subdir == "." {
        return Ok((&head[..repo_end], None));
    }
    if subdir.split('/').any(|c| c == "..") {
        return Err(Error::Ref(format!("bad in-repo path '/{subdir}'")));
    }
    Ok((&head[..repo_end], Some(subdir.to_string())))
}

/// scp-style ssh — `git@host:path`, `host:path`. The `:` lands before
/// the first `/` (a `://` was already ruled out by the caller).
fn is_scp_style(head: &str) -> bool {
    head.split('/')
        .next()
        .is_some_and(|first| first.contains(':'))
}

/// Bare shorthand → a clone URL: `org/repo` → github.com,
/// `host/org/repo` → `https://host/…`; URLs and scp pass through.
fn normalize_clone_url(repo: &str) -> String {
    if repo.contains("://")
        || is_scp_style(repo)
        || repo.starts_with('/')
        || repo.starts_with('.')
        || repo.starts_with('~')
    {
        return repo.to_string();
    }
    let mut segments = repo.split('/');
    let first = segments.next().unwrap_or(repo);
    let two_segments = segments.clone().count() == 1;
    if two_segments && !first.contains('.') {
        format!("https://github.com/{repo}")
    } else {
        format!("https://{repo}")
    }
}

/// `http(s)://….tgz|tar.gz[#sha256:<hex>|#sha256=<hex>]`.
/// URLs are case-sensitive — the extension check is deliberately exact.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn parse_http(input: &str) -> Result<HttpRef, Error> {
    let (url, fragment) = match input.split_once('#') {
        Some((url, fragment)) => (url, Some(fragment)),
        None => (input, None),
    };
    let digest = match fragment {
        None => None,
        Some(fragment) => {
            let hex = fragment
                .strip_prefix("sha256:")
                .or_else(|| fragment.strip_prefix("sha256="))
                .ok_or_else(|| {
                    Error::Ref(format!(
                        "unsupported fragment '#{fragment}' — expected '#sha256:<hex>'"
                    ))
                })?;
            if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::Ref("sha256 digest must be 64 hex chars".to_string()));
            }
            Some(hex.to_ascii_lowercase())
        }
    };
    if !(url.ends_with(".tgz") || url.ends_with(".tar.gz")) {
        return Err(Error::Ref(format!(
            "http refs must point at a .tgz/.tar.gz tarball: {url}"
        )));
    }
    Ok(HttpRef {
        url: url.to_string(),
        digest,
    })
}
