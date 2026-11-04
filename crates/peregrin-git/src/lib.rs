//! peregrin-git — the explicit, librarizable detached-worktree
//! lifecycle (PL-003 P1): `created → kept-for-debug → destroyed`.
//! `peregrin-driver-git` (P2) builds detached worktree-at-sha runs on
//! this; the API stays general enough to use outside runs.
//!
//! Everything goes through a [`Worktrees`] manager bound to a *base
//! dir* — the dir holding both the worktree checkouts and a manifest
//! (`.peregrin-worktrees.json`) recording what was made where. The
//! manifest is what lets [`Worktrees::list`] reconcile records against
//! on-disk reality and surface orphaned dirs a previous process left
//! behind, and what makes [`Worktrees::cleanup`] safe: it destroys
//! every recorded worktree that isn't [`Worktrees::keep`]ed.
//!
//! Git itself sits behind the [`Git`] seam — [`SystemGit`] shells out
//! to the `git` binary under the sanitized
//! [`peregrin_core::child_env`] allowlist; callers can substitute a
//! fake for tests or a hermetic toolchain.
//!
//! Idempotency contract: `create` on an existing destination
//! reconciles (returns the live record, adopts an untracked checkout),
//! `destroy` on a missing or untracked path is `Ok`.

mod git;
mod manifest;

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub use git::{Git, SystemGit};

/// A recorded worktree — what [`Worktrees::create`] returns and what
/// [`Listing::Tracked`] carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worktree {
    /// Absolute path of the checkout dir.
    pub path: PathBuf,
    /// Repo the worktree belongs to (where `git worktree` runs).
    pub repo: PathBuf,
    /// Resolved full commit sha the checkout is detached at.
    pub sha: String,
    /// Kept-for-debug — survives [`Worktrees::cleanup`].
    pub kept: bool,
}

impl AsRef<Path> for Worktree {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// One entry of [`Worktrees::list`] — the reconciliation surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listing {
    /// Manifest-tracked and present on disk.
    Tracked(Worktree),
    /// A dir under the base dir the manifest doesn't know — a
    /// leftover `peregrin-git` didn't create (or lost track of).
    Orphan(PathBuf),
}

impl Listing {
    /// The path on disk, whichever variant.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Tracked(worktree) => &worktree.path,
            Self::Orphan(path) => path,
        }
    }
}

/// Worktree-lifecycle failures.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// `git` ran and exited non-zero.
    #[error(
        "git {} in {} failed ({status}): {stderr}",
        args.join(" "),
        dir.display()
    )]
    Git {
        dir: PathBuf,
        args: Vec<String>,
        status: String,
        stderr: String,
    },
    /// The git binary couldn't be spawned at all.
    #[error("spawn {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: io::Error,
    },
    /// A filesystem operation outside git failed.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: io::Error,
    },
    /// The manifest file exists but doesn't parse.
    #[error("manifest {}: {source}", path.display())]
    Manifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    /// `create`'s destination is occupied by a non-worktree path —
    /// refusing to clobber is the explicit-lifecycle contract.
    #[error("destination {} is occupied by a non-worktree path", .0.display())]
    DestOccupied(PathBuf),
    /// `keep` on a path the manifest doesn't track.
    #[error("{} is not tracked under {}", .0.display(), .1.display())]
    NotTracked(PathBuf, PathBuf),
    /// [`Worktrees::cleanup`] destroyed what it could; these paths
    /// still failed (every entry was attempted).
    #[error(
        "cleanup failed to destroy: {}",
        .0.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
    )]
    Cleanup(Vec<PathBuf>),
}

/// `~` / `~/…` expansion, matching the workspace `home_dir`
/// convention (`$HOME`, else `$USERPROFILE`, else `/`). Non-`~`
/// paths and non-UTF-8 paths pass through untouched.
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

/// Lexically make `path` absolute (no fs access — a dead cwd keeps
/// the relative form rather than erroring). `unwrap_or`'s eagerly
/// evaluated default is deliberate: the `Err` arm is unreachable on a
/// live cwd, so the fallback must be an already-bound variable, not a
/// lazy closure, for its line to stay covered.
fn absolutize(path: &Path) -> PathBuf {
    let fallback = path.to_path_buf();
    std::path::absolute(path).unwrap_or(fallback)
}

fn io_err(context: impl Into<String>) -> impl FnOnce(io::Error) -> Error {
    move |source| Error::Io {
        context: context.into(),
        source,
    }
}

/// The lifecycle manager — one per base dir. Methods load and save
/// the manifest per call: no in-memory copy can drift from what
/// another process (or a crashed run) left on disk.
pub struct Worktrees<G: Git = SystemGit> {
    base: PathBuf,
    git: G,
}

impl Worktrees<SystemGit> {
    /// A manager over the system `git` binary.
    #[must_use]
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self::with_git(base, SystemGit::new())
    }
}

impl<G: Git> Worktrees<G> {
    /// A manager over a custom [`Git`] (fakes, hermetic toolchains).
    /// `base` is `~`-expanded and made absolute so manifest records
    /// stay valid across working directories.
    #[must_use]
    pub fn with_git(base: impl AsRef<Path>, git: G) -> Self {
        Self {
            base: absolutize(&expand_path(base.as_ref())),
            git,
        }
    }

    /// The base dir — checkouts and manifest live under it.
    #[must_use]
    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Resolve `dest` the way the manifest records it: `~`-expanded,
    /// relative paths joined under the base dir, absolute paths kept.
    fn dest_path(&self, dest: &Path) -> PathBuf {
        let dest = expand_path(dest);
        if dest.is_absolute() {
            dest
        } else {
            self.base.join(dest)
        }
    }

    /// `git worktree add --detach <dest> <sha>` plus a manifest
    /// record. `sha` is verified and resolved to a full commit sha
    /// first — a bad ref fails before anything is created.
    ///
    /// Reconciliation (idempotency): a live tracked dir returns its
    /// record; a stale record is dropped and recreated; an existing
    /// checkout the manifest doesn't know is adopted (recording the
    /// head it actually has); an existing non-worktree path is
    /// [`Error::DestOccupied`].
    pub fn create(
        &self,
        repo: impl AsRef<Path>,
        sha: &str,
        dest: impl AsRef<Path>,
    ) -> Result<Worktree, Error> {
        let repo = absolutize(&expand_path(repo.as_ref()));
        let dest = self.dest_path(dest.as_ref());
        let resolved = self
            .git
            .run(
                &repo,
                &[
                    OsStr::new("rev-parse"),
                    OsStr::new("--verify"),
                    OsStr::new(&format!("{sha}^{{commit}}")),
                ],
            )?
            .trim()
            .to_string();

        let mut manifest = manifest::load(&self.base)?;
        if let Some(tracked) = manifest.find(&dest)
            && dest.is_dir()
        {
            return Ok(tracked.to_worktree());
        }
        // Stale record (dir gone) or none at all — `remove` is a
        // no-op on the absent case; recreate below either way.
        manifest.remove(&dest);

        if dest.exists() {
            if dest.is_dir() && dest.join(".git").exists() {
                // A checkout we didn't record — adopt it, recording
                // the head it actually has (falling back to the
                // requested sha when the head can't be read).
                let adopted = match self
                    .git
                    .run(&dest, &[OsStr::new("rev-parse"), OsStr::new("HEAD")])
                {
                    Ok(head) => head.trim().to_string(),
                    Err(_) => resolved.clone(),
                };
                let worktree = Worktree {
                    path: dest,
                    repo,
                    sha: adopted,
                    kept: false,
                };
                manifest.record(&worktree);
                manifest::save(&self.base, &manifest)?;
                return Ok(worktree);
            }
            let occupied = !dest.is_dir()
                || dest
                    .read_dir()
                    .map_err(io_err("read dest dir"))?
                    .next()
                    .is_some();
            if occupied {
                return Err(Error::DestOccupied(dest));
            }
            // An empty dir is a valid `worktree add` target.
        }

        let add = [
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("--detach"),
            dest.as_os_str(),
            OsStr::new(&resolved),
        ];
        if self.git.run(&repo, &add).is_err() {
            // git keeps the admin record of a manually deleted
            // worktree and refuses `add` ("missing but already
            // registered") — prune and retry once; a second failure
            // is the real error.
            let _ = self
                .git
                .run(&repo, &[OsStr::new("worktree"), OsStr::new("prune")]);
            self.git.run(&repo, &add)?;
        }
        let worktree = Worktree {
            path: dest,
            repo,
            sha: resolved,
            kept: false,
        };
        manifest.record(&worktree);
        manifest::save(&self.base, &manifest)?;
        Ok(worktree)
    }

    /// Manifest entries that still exist on disk, plus orphan dirs
    /// under the base dir the manifest doesn't know ([`Listing`]).
    /// Records pointing at deleted dirs are dropped from the manifest
    /// — listing is the reconciliation point.
    pub fn list(&self) -> Result<Vec<Listing>, Error> {
        let mut manifest = manifest::load(&self.base)?;
        let tracked = manifest.entries.len();
        manifest.entries.retain(|e| e.path.is_dir());
        if manifest.entries.len() != tracked {
            manifest::save(&self.base, &manifest)?;
        }
        let mut listings: Vec<Listing> = manifest
            .entries
            .iter()
            .map(|e| Listing::Tracked(e.to_worktree()))
            .collect();
        if self.base.is_dir() {
            // `flatten` skips mid-iteration entry errors — a readdir
            // entry fault is not something reconciliation can act on.
            for entry in fs::read_dir(&self.base)
                .map_err(io_err("read base dir"))?
                .flatten()
            {
                let path = entry.path();
                let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
                let known = manifest.entries.iter().any(|e| e.path == path);
                if is_dir && !known {
                    listings.push(Listing::Orphan(path));
                }
            }
        }
        Ok(listings)
    }

    /// Mark a tracked worktree for-debug — kept worktrees survive
    /// [`Worktrees::cleanup`]. Accepts a [`Worktree`] or any path
    /// resolving to a recorded one.
    pub fn keep(&self, path: impl AsRef<Path>) -> Result<Worktree, Error> {
        let path = self.dest_path(path.as_ref());
        let mut manifest = manifest::load(&self.base)?;
        let Some(entry) = manifest.entries.iter_mut().find(|e| e.path == path) else {
            return Err(Error::NotTracked(path, self.base.clone()));
        };
        entry.kept = true;
        let worktree = entry.to_worktree();
        manifest::save(&self.base, &manifest)?;
        Ok(worktree)
    }

    /// Whether `path` is a tracked, kept worktree. A missing or
    /// corrupt manifest reads as `false` — callers wanting the error
    /// use [`Worktrees::list`] first.
    #[must_use]
    pub fn kept(&self, path: impl AsRef<Path>) -> bool {
        let path = self.dest_path(path.as_ref());
        manifest::load(&self.base).is_ok_and(|m| m.entries.iter().any(|e| e.path == path && e.kept))
    }

    /// `git worktree remove --force` plus the manifest record.
    /// Idempotent: an untracked or already-gone path is `Ok` — a gone
    /// tracked dir gets `git worktree prune` on the repo so git's own
    /// admin records don't linger. When git removal can't run (repo
    /// missing, orphan dir) the checkout dir is removed directly.
    pub fn destroy(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = self.dest_path(path.as_ref());
        let mut manifest = manifest::load(&self.base)?;
        let tracked = manifest.find(&path).cloned();
        if path.exists() {
            let git_removed = tracked.as_ref().is_some_and(|e| {
                self.git
                    .run(
                        &e.repo,
                        &[
                            OsStr::new("worktree"),
                            OsStr::new("remove"),
                            OsStr::new("--force"),
                            path.as_os_str(),
                        ],
                    )
                    .is_ok()
            });
            if !git_removed {
                fs::remove_dir_all(&path).map_err(io_err("remove worktree dir"))?;
            }
        } else if let Some(entry) = &tracked {
            // Dir already gone — drop git's stale admin record too
            // (best effort; the record being gone already is fine).
            let _ = self
                .git
                .run(&entry.repo, &[OsStr::new("worktree"), OsStr::new("prune")]);
        }
        if tracked.is_some() {
            manifest.remove(&path);
            manifest::save(&self.base, &manifest)?;
        }
        Ok(())
    }

    /// The run-loop teardown: destroy every tracked, non-kept
    /// worktree. Orphan dirs are reported by [`Worktrees::list`], not
    /// touched here — deleting what we never recorded isn't
    /// cleanup's call. All entries are attempted; failures return
    /// aggregated as [`Error::Cleanup`], successes as the destroyed
    /// paths.
    pub fn cleanup(&self) -> Result<Vec<PathBuf>, Error> {
        let manifest = manifest::load(&self.base)?;
        let mut destroyed = Vec::new();
        let mut failures = Vec::new();
        for entry in manifest.entries.iter().filter(|e| !e.kept) {
            if self.destroy(&entry.path).is_err() {
                failures.push(entry.path.clone());
            } else {
                destroyed.push(entry.path.clone());
            }
        }
        if failures.is_empty() {
            Ok(destroyed)
        } else {
            Err(Error::Cleanup(failures))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::error::Error as _;

    use super::*;
    use tempfile::TempDir;

    /// A canned `git`: returns a fixed sha for `rev-parse`, empty
    /// stdout otherwise, or `Error::Git` when `fail_on` appears in the
    /// args. Coverage counts generic instantiations per test binary —
    /// the integration suite covers its own copies, so this binary
    /// needs its own walk over every public path.
    struct StubGit(Option<&'static str>);

    impl Git for StubGit {
        fn run(&self, dir: &Path, args: &[&OsStr]) -> Result<String, Error> {
            if self
                .0
                .is_some_and(|needle| args.iter().any(|a| a.to_str() == Some(needle)))
            {
                return Err(Error::Git {
                    dir: dir.to_path_buf(),
                    args: args
                        .iter()
                        .map(|a| a.to_string_lossy().into_owned())
                        .collect(),
                    status: "exit status: 1".to_string(),
                    stderr: "stub fail".to_string(),
                });
            }
            let is_rev_parse = args.iter().any(|a| a.to_str() == Some("rev-parse"));
            Ok(if is_rev_parse {
                "feedbeef\n".to_string()
            } else {
                String::new()
            })
        }
    }

    fn corrupt_manifest(base: &Path) {
        fs::create_dir_all(base).unwrap();
        fs::write(base.join(".peregrin-worktrees.json"), "{ nope").unwrap();
    }

    /// Every public method over the happy/stubbed paths — `create`
    /// shapes (fresh, reconcile, stale, adopt, occupied, empty-dir),
    /// `keep`/`kept`, `destroy` (every arm), `list` (tracked +
    /// orphan), `cleanup`.
    #[test]
    fn api_walk() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("base");
        let w = Worktrees::with_git(&base, StubGit(None));
        // The `Worktrees<SystemGit>` method instantiations in this
        // binary: each public call once on an empty base.
        let system = Worktrees::new(tmp.path().join("other"));
        assert!(system.base().ends_with("other"));
        assert_eq!(system.list().unwrap().len(), 0);
        let nothing = tmp.path().join("nothing");
        assert!(!system.kept(&nothing));
        assert!(matches!(system.keep(&nothing), Err(Error::NotTracked(..))));
        system.destroy(&nothing).unwrap();
        assert_eq!(system.cleanup().unwrap().len(), 0);
        assert!(w.base().ends_with("base"));

        // Path helpers.
        let home = expand_path(Path::new("~"));
        assert_eq!(expand_path(Path::new("~/x")), home.join("x"));
        assert_eq!(expand_path(Path::new("rel")), PathBuf::from("rel"));

        // create: fresh record.
        let wt = w.create("/repo", "HEAD", "run-1").unwrap();
        assert_eq!(wt.sha, "feedbeef");
        assert_eq!(wt.repo, PathBuf::from("/repo"));
        assert!(!w.kept(&wt));

        // create: reconcile a live tracked dir.
        fs::create_dir_all(&wt.path).unwrap();
        assert_eq!(w.create("/repo", "HEAD", "run-1").unwrap(), wt);

        // create: stale record (dir gone) drops and re-adds.
        fs::remove_dir_all(&wt.path).unwrap();
        assert_eq!(w.create("/repo", "HEAD", "run-1").unwrap(), wt);

        // create: adopt an untracked checkout.
        let stray = base.join("stray");
        fs::create_dir_all(&stray).unwrap();
        fs::write(stray.join(".git"), "x").unwrap();
        assert_eq!(w.create("/repo", "HEAD", &stray).unwrap().sha, "feedbeef");

        // create: occupied — a file, then a nonempty non-checkout dir.
        let file_dest = base.join("file");
        fs::write(&file_dest, "x").unwrap();
        assert!(matches!(
            w.create("/repo", "HEAD", &file_dest),
            Err(Error::DestOccupied(_))
        ));
        let full_dir = base.join("full");
        fs::create_dir_all(&full_dir).unwrap();
        fs::write(full_dir.join("x"), "x").unwrap();
        assert!(matches!(
            w.create("/repo", "HEAD", &full_dir),
            Err(Error::DestOccupied(_))
        ));

        // create: existing empty dir fills fine.
        let empty = base.join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert_eq!(w.create("/repo", "HEAD", &empty).unwrap().sha, "feedbeef");

        // keep / kept / NotTracked.
        let wt = w.keep(&wt).unwrap();
        assert!(wt.kept);
        assert!(w.kept(&wt));
        assert!(matches!(
            w.keep(base.join("nope")),
            Err(Error::NotTracked(..))
        ));

        // list: tracked entries plus orphans (plain files ignored).
        // `wt`'s dir is made present — a missing one would be
        // reconciled out of the manifest instead.
        fs::create_dir_all(&wt.path).unwrap();
        let orphan = base.join("orphan");
        fs::create_dir_all(&orphan).unwrap();
        let listings = w.list().unwrap();
        assert!(listings.contains(&Listing::Tracked(wt.clone())));
        assert!(listings.contains(&Listing::Orphan(orphan.clone())));
        assert!(listings.contains(&Listing::Orphan(full_dir)));
        let _ = listings.clone();
        let _ = format!("{listings:?}");

        // destroy: tracked, dir present, git remove ok.
        let live = w.create("/repo", "HEAD", "live").unwrap();
        fs::create_dir_all(&live.path).unwrap();
        w.destroy(&live).unwrap();
        // destroy: tracked, dir already gone → prune arm.
        w.destroy(&wt).unwrap();
        let gone = w.create("/repo", "HEAD", "gone").unwrap();
        w.destroy(&gone).unwrap();
        // destroy: orphan dir → fs fallback.
        w.destroy(&orphan).unwrap();
        // destroy: never tracked → Ok.
        w.destroy(base.join("nothing")).unwrap();
        // destroy: undestroyable path → io error. By-value `PathBuf`
        // args exercise that instantiation too.
        let file_arg = file_dest.clone();
        assert!(matches!(w.destroy(file_arg), Err(Error::Io { .. })));
        assert!(matches!(w.destroy(&file_dest), Err(Error::Io { .. })));

        // cleanup: destroys every unkept record, spares a kept one,
        // reports them, idempotent.
        let doomed = w.create("/repo", "HEAD", "doomed").unwrap();
        fs::create_dir_all(&doomed.path).unwrap();
        let keeper = w.create("/repo", "HEAD", "keeper").unwrap();
        fs::create_dir_all(&keeper.path).unwrap();
        let keeper = w.keep(&keeper).unwrap();
        let destroyed = w.cleanup().unwrap();
        assert_eq!(destroyed.len(), 3);
        for path in [&stray, &empty, &doomed.path] {
            assert!(destroyed.contains(path));
        }
        assert!(w.kept(&keeper));
        assert!(keeper.path.exists());
        assert_eq!(w.cleanup().unwrap().len(), 0);
    }

    /// The `git`-failure edges a StubGit can reach: bad ref, failed
    /// add (incl. the prune-retry), failed remove falling back to fs.
    #[test]
    fn git_failure_arms() {
        let tmp = TempDir::new().unwrap();
        // `rev-parse` fails → the very first `?` in create.
        let bad_ref = Worktrees::with_git(tmp.path().join("a"), StubGit(Some("rev-parse")));
        assert!(matches!(
            bad_ref.create("/r", "HEAD", "d"),
            Err(Error::Git { .. })
        ));
        // `add` fails twice (prune between) → the retry's `?`.
        let bad_add = Worktrees::with_git(tmp.path().join("b"), StubGit(Some("--detach")));
        assert!(matches!(
            bad_add.create("/r", "HEAD", "d"),
            Err(Error::Git { .. })
        ));
        // `remove` fails → fs fallback removes the dir.
        let bad_rm = Worktrees::with_git(tmp.path().join("c"), StubGit(Some("remove")));
        let wt = bad_rm.create("/r", "HEAD", "x").unwrap();
        fs::create_dir_all(&wt.path).unwrap();
        bad_rm.destroy(&wt).unwrap();
        assert!(!wt.path.exists());
        assert_eq!(bad_rm.list().unwrap().len(), 0);
    }

    /// `destroy` on a *tracked* path that is a plain file: git
    /// removal can't run and `remove_dir_all` errors — the cleanup
    /// aggregate arm too.
    #[test]
    fn destroy_and_cleanup_failure_arms() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("base");
        let w = Worktrees::with_git(&base, StubGit(Some("remove")));
        let stuck = w.create("/repo", "HEAD", "stuck").unwrap();
        fs::write(&stuck.path, "not a dir").unwrap();
        assert!(matches!(w.destroy(&stuck), Err(Error::Io { .. })));
        let err = w.cleanup().unwrap_err();
        assert!(matches!(err, Error::Cleanup(_)));
        assert!(err.to_string().contains("stuck"));
    }

    /// Corrupt and unreadable manifests hit `load`'s error arms in
    /// every op.
    #[test]
    fn manifest_error_arms() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("base");
        corrupt_manifest(&base);
        let w = Worktrees::with_git(&base, StubGit(None));
        let p = base.join("x");
        assert!(matches!(w.list(), Err(Error::Manifest { .. })));
        assert!(matches!(
            w.create("/r", "H", &p),
            Err(Error::Manifest { .. })
        ));
        assert!(matches!(w.keep(&p), Err(Error::Manifest { .. })));
        assert!(matches!(w.destroy(&p), Err(Error::Manifest { .. })));
        assert!(matches!(w.cleanup(), Err(Error::Manifest { .. })));
        assert!(!w.kept(&p));

        // Manifest path that isn't a file → the io arm of `load`.
        let base2 = tmp.path().join("b2");
        fs::create_dir_all(base2.join(".peregrin-worktrees.json")).unwrap();
        let w2 = Worktrees::with_git(&base2, StubGit(None));
        assert!(matches!(w2.list(), Err(Error::Io { .. })));
    }

    /// Read-only manifest: `load` parses fine, `save` fails at
    /// `File::create` — every op's save `?`.
    #[cfg(unix)]
    #[test]
    fn save_error_arms() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("base");
        let w = Worktrees::with_git(&base, StubGit(None));
        // Seed a readable-but-unwritable manifest with one record.
        let wt = w.create("/repo", "HEAD", "x").unwrap();
        let manifest = base.join(".peregrin-worktrees.json");
        let mut perms = fs::metadata(&manifest).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&manifest, perms).unwrap();

        // keep → save fails; adopt → save fails; destroy on a gone
        // dir → save fails; stale rewrite in list → save fails.
        assert!(matches!(w.keep(&wt), Err(Error::Io { .. })));
        let stray = base.join("stray");
        fs::create_dir_all(&stray).unwrap();
        fs::write(stray.join(".git"), "x").unwrap();
        assert!(matches!(
            w.create("/repo", "HEAD", &stray),
            Err(Error::Io { .. })
        ));
        assert!(matches!(w.destroy(&wt), Err(Error::Io { .. })));
        assert!(matches!(w.list(), Err(Error::Io { .. })));

        // Unwritable parent → `create_dir_all` fails.
        let locked = tmp.path().join("locked");
        fs::create_dir_all(&locked).unwrap();
        let mut perms = fs::metadata(&locked).unwrap().permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&locked, perms.clone()).unwrap();
        let w2 = Worktrees::with_git(locked.join("base"), StubGit(None));
        let err = w2.create("/repo", "HEAD", tmp.path().join("elsewhere").join("d"));
        let mut perms = fs::metadata(&locked).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&locked, perms).unwrap();
        assert!(matches!(err, Err(Error::Io { .. })));
    }

    /// Unreadable base dir → `read_dir` fails in `list`.
    #[cfg(unix)]
    #[test]
    fn list_read_dir_arm() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("base");
        fs::create_dir_all(&base).unwrap();
        let mut perms = fs::metadata(&base).unwrap().permissions();
        perms.set_mode(0o111);
        fs::set_permissions(&base, perms.clone()).unwrap();
        let w = Worktrees::with_git(&base, StubGit(None));
        let err = w.list();
        let mut perms = fs::metadata(&base).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&base, perms).unwrap();
        assert!(matches!(err, Err(Error::Io { .. })));
    }

    /// Unreadable dest dir → `read_dir` fails in `create`'s occupied
    /// check.
    #[cfg(unix)]
    #[test]
    fn create_dest_read_dir_arm() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("base");
        let dest = base.join("unreadable");
        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("x"), "x").unwrap();
        let mut perms = fs::metadata(&dest).unwrap().permissions();
        perms.set_mode(0o111);
        fs::set_permissions(&dest, perms.clone()).unwrap();
        let w = Worktrees::with_git(&base, StubGit(None));
        let err = w.create("/repo", "HEAD", &dest);
        let mut perms = fs::metadata(&dest).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).unwrap();
        assert!(matches!(err, Err(Error::Io { .. })));
    }

    /// Every error variant's Display + Debug + source — the
    /// thiserror-generated match arms only count when exercised.
    #[test]
    fn error_surface() {
        let io = || io::Error::other("x");
        let serde = || serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let errs = [
            Error::Git {
                dir: PathBuf::from("/d"),
                args: vec!["a".to_string()],
                status: "1".to_string(),
                stderr: "e".to_string(),
            },
            Error::Spawn {
                program: "git".to_string(),
                source: io(),
            },
            Error::Io {
                context: "ctx".to_string(),
                source: io(),
            },
            Error::Manifest {
                path: PathBuf::from("/m"),
                source: serde(),
            },
            Error::DestOccupied(PathBuf::from("/x")),
            Error::NotTracked(PathBuf::from("/x"), PathBuf::from("/b")),
            Error::Cleanup(vec![PathBuf::from("/x")]),
        ];
        for err in &errs {
            let _ = format!("{err:?}");
            let _ = err.to_string();
            let _ = err.source();
        }
    }

    /// Re-exec — `home_dir`'s `$USERPROFILE`-then-`/` fallbacks need
    /// `HOME` unset, and edition-2024 `remove_var` is unsafe; the
    /// child process env is the clean seam (same pattern as
    /// peregrin-driver-shell's env tests).
    #[test]
    fn expand_path_homeless() {
        if std::env::var_os("PERE_HOMELESS").is_some() {
            assert_eq!(expand_path(Path::new("~")), PathBuf::from("/"));
            assert_eq!(expand_path(Path::new("~/x")), PathBuf::from("/x"));
            return;
        }
        // `--exact` needs the qualified name (`tests::…`) — the
        // thread name carries it.
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([std::thread::current().name().unwrap(), "--exact"])
            .env("PERE_HOMELESS", "1")
            .env_remove("HOME")
            .env_remove("USERPROFILE")
            .status()
            .unwrap();
        assert!(status.success());
    }
}
