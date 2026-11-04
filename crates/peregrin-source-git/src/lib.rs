//! peregrin-source-git — the `git:` arm of ADR-0079's ref grammar:
//! any clone URL (https/ssh/`git@`/shorthand `org/repo` → github.com,
//! already normalized by [`peregrin_source::parse_ref`]), `//subdir`,
//! and `#<branch|tag|sha>` pins. `resolve` pins the ref to a commit
//! sha via `ls-remote` — the resolved sha is the content key
//! (`git-<sha>`), so a warm cache makes `install` offline-free.
//!
//! Git work goes through the [`Git`] seam — [`SystemGit`] shells out;
//! tests substitute a stub for the error arms and drive real
//! `file://` fixtures for the happy paths.

use std::ffi::OsStr;
use std::fs;
use std::path::Path;

use peregrin_bundle::copy_tree;
use peregrin_git::{Git, SystemGit};
use peregrin_source::{Error, Fetched, GitRef, Resolved, Source, SourceRef, expand_path};

/// peregrin-bundle failures fold into the port's [`Error::Failed`].
fn be(e: &peregrin_bundle::Error) -> Error {
    Error::Failed(e.to_string())
}

/// [`Source`] over git remotes — generic over the [`Git`] seam.
pub struct GitSource<G: Git = SystemGit> {
    git: G,
}

impl GitSource<SystemGit> {
    /// The git source over the system `git` binary.
    #[must_use]
    pub fn new() -> Self {
        Self {
            git: SystemGit::new(),
        }
    }
}

impl Default for GitSource<SystemGit> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: Git> GitSource<G> {
    /// A git source over a custom [`Git`] (fakes, hermetic toolchains).
    #[must_use]
    pub fn with_git(git: G) -> Self {
        Self { git }
    }
}

/// 40 lowercase-hex chars — an already-fully-pinned sha needs no
/// `ls-remote`.
fn is_full_sha(rev: &str) -> bool {
    rev.len() == 40
        && rev
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// A hex prefix (7..40 chars) resolves at fetch — the clone carries
/// the object and `rev-parse` completes it.
fn is_hex_prefix(rev: &str) -> bool {
    rev.len() >= 7
        && rev.len() < 40
        && rev
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// `~`/`./`-leading clone paths expand like fs refs.
fn expand_url(url: &str) -> Result<String, Error> {
    if !(url.starts_with('~') || url.starts_with('/')) {
        return Ok(url.to_string());
    }
    expand_path(Path::new(url))
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| Error::Failed(format!("non-UTF-8 clone path '{url}'")))
}

/// `ls-remote` stdout → `(refname → sha)` map. Lines are `sha\tref`;
/// anything else is skipped.
fn parse_ls_remote(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|line| {
            let (sha, refname) = line.split_once('\t')?;
            Some((refname.trim().to_string(), sha.trim().to_string()))
        })
        .collect()
}

impl<G: Git> GitSource<G> {
    /// `git ls-remote <url>` → the commit sha `rev` (or HEAD) points
    /// at. Tags prefer the peeled `^{}` line; a sha-looking `rev`
    /// never reaches here (callers pin it directly).
    fn resolve_remote(&self, url: &str, rev: Option<&str>) -> Result<String, Error> {
        let cwd = Path::new(".");
        let out = self
            .git
            .run(cwd, &[OsStr::new("ls-remote"), OsStr::new(url)])
            .map_err(|e| Error::Failed(format!("ls-remote '{url}': {e}")))?;
        let refs = parse_ls_remote(&out);
        let find = |name: &str| {
            refs.iter()
                .find(|(refname, _)| refname == name)
                .map(|(_, sha)| sha.clone())
        };
        match rev {
            None => find("HEAD")
                .ok_or_else(|| Error::Failed(format!("remote '{url}' advertises no HEAD"))),
            Some(rev) => find(&format!("refs/tags/{rev}^{{}}"))
                .or_else(|| find(&format!("refs/tags/{rev}")))
                .or_else(|| find(&format!("refs/heads/{rev}")))
                .or_else(|| find(&format!("refs/pull/{rev}/head")))
                .ok_or_else(|| Error::Failed(format!("ref '{rev}' not found on '{url}'"))),
        }
    }
}

impl<G: Git> Source for GitSource<G> {
    /// Pin the ref: a 40-hex `rev` is the sha already; a shorter hex
    /// prefix defers to fetch; names and HEAD go through `ls-remote`.
    fn resolve(&self, reference: &SourceRef) -> Result<Resolved, Error> {
        let SourceRef::Git(GitRef { url, subdir, rev }) = reference else {
            return Err(Error::Failed(
                "GitSource only resolves git refs".to_string(),
            ));
        };
        let url = expand_url(url)?;
        let sha = match rev.as_deref() {
            Some(rev) if is_full_sha(rev) => Some(rev.to_string()),
            Some(rev) if is_hex_prefix(rev) => None,
            other => Some(self.resolve_remote(&url, other)?),
        };
        Ok(Resolved::Git {
            url,
            subdir: subdir.clone(),
            rev: rev.clone(),
            sha,
        })
    }

    /// Clone → `rev-parse`/`checkout` the pin → copy the (sub)tree
    /// into `dest_dir` minus `.git/`. The checkout happens inside the
    /// caller's scratch dir, so nothing leaks on failure.
    fn fetch(&self, resolved: &Resolved, dest_dir: &Path) -> Result<Fetched, Error> {
        let Resolved::Git {
            url,
            subdir,
            rev,
            sha,
        } = resolved
        else {
            return Err(Error::Failed(
                "GitSource only fetches git specs".to_string(),
            ));
        };
        fs::create_dir_all(dest_dir)
            .map_err(Error::io(format!("create {}", dest_dir.display())))?;
        let clone = dest_dir.join(".clone");
        let clone_run = |args: &[&OsStr]| -> Result<String, Error> {
            self.git
                .run(&clone, args)
                .map_err(|e| Error::Failed(format!("git clone '{url}': {e}")))
        };
        self.git
            .run(
                dest_dir,
                &[
                    OsStr::new("clone"),
                    OsStr::new("--quiet"),
                    OsStr::new(url),
                    clone.as_os_str(),
                ],
            )
            .map_err(|e| Error::Failed(format!("clone '{url}': {e}")))?;
        let want = sha.as_deref().or(rev.as_deref()).unwrap_or("HEAD");
        let want_commit = format!("{want}^{{commit}}");
        let sha = clone_run(&[
            OsStr::new("rev-parse"),
            OsStr::new("--verify"),
            OsStr::new(&want_commit),
        ])
        .map_err(|e| Error::Failed(format!("ref '{want}' not on '{url}': {e}")))?
        .trim()
        .to_string();
        clone_run(&[
            OsStr::new("checkout"),
            OsStr::new("--quiet"),
            OsStr::new(&sha),
        ])?;
        let payload = subdir
            .as_ref()
            .map_or_else(|| clone.clone(), |s| clone.join(s));
        if !payload.is_dir() {
            return Err(Error::Failed(format!(
                "subdir '{}' not in {url}@{sha}",
                subdir.as_deref().unwrap_or_default()
            )));
        }
        copy_tree(&payload, dest_dir).map_err(|e| be(&e))?;
        fs::remove_dir_all(&clone).map_err(Error::io(format!("clear {}", clone.display())))?;
        Ok(Fetched {
            dir: dest_dir.to_path_buf(),
            resolved: sha.clone(),
            content_key: format!("git-{sha}"),
        })
    }
}
