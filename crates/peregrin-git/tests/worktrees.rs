//! Lifecycle tests over real temp repos — `peregrin-git` owns
//! process-level git behavior, so the suite shells out rather than
//! faking the seam it exists to wrap. Each test gets its own
//! `TempDir`; nothing mutates process-global state except the
//! re-exec'd `homeless` probe, which runs the env scrub in a child
//! process.
#![allow(clippy::unwrap_used)]

use std::error::Error as _;
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use peregrin_git::{Error, Listing, SystemGit, Worktrees, expand_path};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repo with two commits — `(repo_dir, first_sha, head_sha)`.
fn fixture_repo(root: &Path) -> (PathBuf, String, String) {
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    fs::write(repo.join("f"), "one").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "one"]);
    let first = git(&repo, &["rev-parse", "HEAD"]);
    fs::write(repo.join("f"), "two").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "two"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    (repo, first, head)
}

fn worktree_list(repo: &Path) -> String {
    git(repo, &["worktree", "list", "--porcelain"])
}

/// A manifest file with `entries` hand-written — for planting state
/// `create` could never produce (the cleanup-failure fixture).
fn write_manifest(base: &Path, entries: &serde_json::Value) {
    fs::create_dir_all(base).unwrap();
    let manifest = serde_json::json!({ "entries": entries });
    fs::write(
        base.join(".peregrin-worktrees.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn create_detaches_at_exact_sha() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);

    let wt = w.create(&repo, &first, "run-1").unwrap();
    assert_eq!(wt.path, base.join("run-1"));
    assert_eq!(wt.repo, repo);
    assert_eq!(wt.sha, first);
    assert_ne!(wt.sha, head);
    assert!(!wt.kept);
    // Detached at the requested sha — the second commit's file
    // content isn't there.
    assert_eq!(git(&wt.path, &["rev-parse", "HEAD"]), first);
    assert_eq!(fs::read_to_string(wt.path.join("f")).unwrap(), "one");
    assert!(worktree_list(&repo).contains("run-1"));

    // And the manifest records it for `list`.
    let listings = w.list().unwrap();
    assert_eq!(listings, vec![Listing::Tracked(wt.clone())]);
    assert_eq!(listings[0].path(), wt.path.as_path());
    let _ = format!("{wt:?}");
    let _ = listings.clone();
    let _ = format!("{listings:?}");
}

#[test]
fn create_reconciles_existing_dest() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let w = Worktrees::new(tmp.path().join("worktrees"));

    let wt = w.create(&repo, &first, "run-1").unwrap();
    let again = w.create(&repo, &first, "run-1").unwrap();
    assert_eq!(again, wt);
    assert_eq!(w.list().unwrap().len(), 1);
}

#[test]
fn create_recreates_stale_record() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);

    let wt = w.create(&repo, &first, "run-1").unwrap();
    fs::remove_dir_all(&wt.path).unwrap();
    let again = w.create(&repo, &first, "run-1").unwrap();
    assert!(again.path.is_dir());
    assert_eq!(git(&again.path, &["rev-parse", "HEAD"]), first);
    assert_eq!(w.list().unwrap().len(), 1);
}

#[test]
fn create_adopts_untracked_checkout() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let dest = base.join("stray");
    fs::create_dir_all(&base).unwrap();
    // A worktree `peregrin-git` never recorded — create() must adopt
    // it, not fail or clobber. The record keeps the checkout's real
    // head (`first`), not the requested `head`.
    git(
        &repo,
        &[
            "worktree",
            "add",
            "--detach",
            dest.to_str().unwrap(),
            &first,
        ],
    );
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &head, &dest).unwrap();
    assert_eq!(wt.sha, first);
    assert!(w.list().unwrap().contains(&Listing::Tracked(wt.clone())));
    w.destroy(&wt).unwrap();
    assert!(!dest.exists());
    assert!(!worktree_list(&repo).contains("stray"));
}

#[test]
fn create_adopts_checkout_when_head_unreadable() {
    let tmp = TempDir::new().unwrap();
    let (repo, _, head) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let dest = base.join("fake-checkout");
    fs::create_dir_all(&dest).unwrap();
    // `.git` exists but isn't a real checkout — `rev-parse HEAD`
    // fails and adoption falls back to the requested sha.
    fs::write(dest.join(".git"), "garbage").unwrap();
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &head, &dest).unwrap();
    assert_eq!(wt.sha, head);
}

#[test]
fn create_refuses_occupied_dest() {
    let tmp = TempDir::new().unwrap();
    let (repo, _, head) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);

    // A non-empty dir that isn't a checkout.
    let occupied_dir = base.join("data");
    fs::create_dir_all(&occupied_dir).unwrap();
    fs::write(occupied_dir.join("keep.me"), "x").unwrap();
    let err = w.create(&repo, &head, &occupied_dir).unwrap_err();
    assert!(matches!(err, Error::DestOccupied(_)));
    assert!(err.to_string().contains("data"));
    let _ = format!("{err:?}");

    // A plain file at the destination.
    let file_dest = base.join("file");
    fs::write(&file_dest, "x").unwrap();
    let err = w.create(&repo, &head, &file_dest).unwrap_err();
    assert!(matches!(err, Error::DestOccupied(_)));
}

#[test]
fn create_rejects_bad_ref() {
    let tmp = TempDir::new().unwrap();
    let (repo, _, _) = fixture_repo(tmp.path());
    let w = Worktrees::new(tmp.path().join("worktrees"));
    let err = w
        .create(&repo, "0000000000000000000000000000000000000000", "x")
        .unwrap_err();
    assert!(matches!(err, Error::Git { .. }));
    assert!(err.to_string().contains("rev-parse"));
    assert!(err.source().is_none());
    let _ = format!("{err:?}");
    // Nothing was created or recorded.
    assert_eq!(w.list().unwrap().len(), 0);
}

#[test]
fn create_fills_existing_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let dest = base.join("empty");
    fs::create_dir_all(&dest).unwrap();
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &first, &dest).unwrap();
    assert_eq!(git(&wt.path, &["rev-parse", "HEAD"]), first);
}

#[test]
fn keep_marks_for_debug_and_persists() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &first, "run-1").unwrap();
    assert!(!w.kept(&wt));

    let kept = w.keep(&wt).unwrap();
    assert!(kept.kept);
    assert!(w.kept(&wt));

    // Persisted on disk — a fresh manager over the same base sees it.
    let w2 = Worktrees::new(&base);
    assert!(w2.kept(&wt));
}

#[test]
fn keep_rejects_untracked() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let err = w.keep(base.join("never-made")).unwrap_err();
    assert!(matches!(err, Error::NotTracked(..)));
    assert!(err.to_string().contains("never-made"));
    assert!(err.to_string().contains("worktrees"));
    let _ = format!("{err:?}");
}

#[test]
fn kept_false_without_manifest_or_record() {
    let tmp = TempDir::new().unwrap();
    let w = Worktrees::new(tmp.path().join("worktrees"));
    assert!(!w.kept(tmp.path().join("anything")));
}

#[test]
fn destroy_removes_checkout_and_record() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &first, "run-1").unwrap();

    w.destroy(&wt).unwrap();
    assert!(!wt.path.exists());
    assert_eq!(w.list().unwrap().len(), 0);
    assert!(!worktree_list(&repo).contains("run-1"));

    // Idempotent: same destroy again is Ok, so is a never-known path.
    w.destroy(&wt).unwrap();
    w.destroy(base.join("never-made")).unwrap();
}

#[test]
fn destroy_prunes_repo_record_when_dir_gone() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &first, "run-1").unwrap();

    // Dir deleted out from under us — destroy drops the manifest
    // record AND prunes git's stale admin entry.
    fs::remove_dir_all(&wt.path).unwrap();
    w.destroy(&wt).unwrap();
    assert_eq!(w.list().unwrap().len(), 0);
    assert!(!worktree_list(&repo).contains("run-1"));
}

#[test]
fn destroy_orphan_uses_fs_removal() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    let orphan = base.join("leftover");
    fs::create_dir_all(&orphan).unwrap();
    fs::write(orphan.join("artifact"), "x").unwrap();
    let w = Worktrees::new(&base);
    w.destroy(&orphan).unwrap();
    assert!(!orphan.exists());
}

#[test]
fn destroy_falls_back_to_fs_when_git_remove_fails() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &first, "run-1").unwrap();

    // Repo gone — `git worktree remove` can't even spawn in it, so
    // the dir comes out via fs removal and the record still drops.
    fs::remove_dir_all(&repo).unwrap();
    w.destroy(&wt).unwrap();
    assert!(!wt.path.exists());
    assert_eq!(w.list().unwrap().len(), 0);
}

#[test]
fn destroy_errors_when_dir_undestroyable() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    let stuck = base.join("stuck");
    fs::create_dir_all(&base).unwrap();
    fs::write(&stuck, "not a dir").unwrap();
    write_manifest(
        &base,
        &serde_json::json!([{
            "path": stuck.clone(),
            "repo": "/nonexistent-repo",
            "sha": "s",
            "kept": false,
        }]),
    );
    let w = Worktrees::new(&base);
    let err = w.destroy(&stuck).unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
    assert!(err.to_string().contains("remove worktree dir"));
    assert!(err.source().is_some());
    let _ = format!("{err:?}");
}

#[test]
fn list_reports_tracked_and_orphans_and_prunes_stale() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let wt = w.create(&repo, &first, "tracked").unwrap();
    let stale = w.create(&repo, &first, "stale").unwrap();
    fs::remove_dir_all(&stale.path).unwrap();
    // Orphans: an empty stray dir and one with contents; a plain
    // file must NOT be reported.
    let orphan = base.join("orphan");
    fs::create_dir_all(&orphan).unwrap();
    fs::write(orphan.join("x"), "x").unwrap();
    fs::write(base.join("just-a-file"), "x").unwrap();

    let listings = w.list().unwrap();
    assert_eq!(listings.len(), 2);
    assert!(listings.contains(&Listing::Tracked(wt)));
    assert!(listings.contains(&Listing::Orphan(orphan.clone())));

    // The stale record was reconciled out of the manifest — a
    // follow-up list on a fresh manager doesn't resurface it.
    let w2 = Worktrees::new(&base);
    let listings = w2.list().unwrap();
    assert_eq!(listings.len(), 2);
    assert!(listings.iter().all(|l| l.path() != stale.path));
    assert!(listings.iter().any(|l| l.path() == orphan));
}

#[test]
fn list_empty_when_base_missing() {
    let tmp = TempDir::new().unwrap();
    let w = Worktrees::new(tmp.path().join("no-base"));
    assert_eq!(w.list().unwrap().len(), 0);
}

#[test]
fn list_surfaces_corrupt_manifest() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join(".peregrin-worktrees.json"), "{ not json").unwrap();
    let w = Worktrees::new(&base);
    let err = w.list().unwrap_err();
    assert!(matches!(err, Error::Manifest { .. }));
    assert!(err.to_string().contains(".peregrin-worktrees.json"));
    assert!(err.source().is_some());
    let _ = format!("{err:?}");
    // `kept` swallows the load failure into `false`.
    assert!(!w.kept(base.join("x")));
}

#[test]
fn manifest_path_as_dir_is_an_io_error() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    fs::create_dir_all(base.join(".peregrin-worktrees.json")).unwrap();
    let w = Worktrees::new(&base);
    let err = w.list().unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
    assert!(err.to_string().contains(".peregrin-worktrees.json"));
}

#[cfg(unix)]
#[test]
fn save_errors_when_base_uncreatable() {
    // Base's parent isn't writable — `git worktree add` lands `dest`
    // elsewhere, then `create_dir_all(base)` fails.
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let locked = tmp.path().join("locked");
    fs::create_dir_all(&locked).unwrap();
    let base = locked.join("worktrees");
    let dest = tmp.path().join("elsewhere").join("run-1");
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&locked, perms.clone()).unwrap();
    let w = Worktrees::new(&base);
    let err = w.create(&repo, &first, &dest).unwrap_err();
    // Restore writability so TempDir cleanup works.
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&locked, perms).unwrap();
    assert!(matches!(err, Error::Io { .. }));
    assert!(dest.exists());
}

#[cfg(unix)]
#[test]
fn save_errors_when_manifest_uncreatable() {
    // Base exists but isn't writable — the manifest read misses
    // (NotFound → empty), `git worktree add` lands `dest` elsewhere,
    // then `File::create` fails.
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    fs::create_dir_all(&base).unwrap();
    let dest = tmp.path().join("elsewhere").join("run-1");
    let mut perms = fs::metadata(&base).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&base, perms.clone()).unwrap();
    let w = Worktrees::new(&base);
    let err = w.create(&repo, &first, &dest).unwrap_err();
    let mut perms = fs::metadata(&base).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&base, perms).unwrap();
    assert!(matches!(err, Error::Io { .. }));
    assert!(dest.exists());
}

#[test]
fn corrupt_manifest_surfaces_in_every_op() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join(".peregrin-worktrees.json"), "{ not json").unwrap();
    let w = Worktrees::new(&base);
    let path = base.join("x");
    assert!(matches!(
        w.create(&repo, &first, &path).unwrap_err(),
        Error::Manifest { .. }
    ));
    assert!(matches!(w.keep(&path).unwrap_err(), Error::Manifest { .. }));
    assert!(matches!(
        w.destroy(&path).unwrap_err(),
        Error::Manifest { .. }
    ));
    assert!(matches!(w.cleanup().unwrap_err(), Error::Manifest { .. }));
}

#[cfg(unix)]
#[test]
fn list_surfaces_unreadable_base() {
    // Search-only base dir (no read) — `read_dir` fails EACCES.
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    fs::create_dir_all(&base).unwrap();
    let mut perms = fs::metadata(&base).unwrap().permissions();
    perms.set_mode(0o111);
    fs::set_permissions(&base, perms.clone()).unwrap();
    let w = Worktrees::new(&base);
    let err = w.list().unwrap_err();
    let mut perms = fs::metadata(&base).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&base, perms).unwrap();
    assert!(matches!(err, Error::Io { .. }));
}

#[cfg(unix)]
#[test]
fn create_surfaces_unreadable_dest() {
    // `dest` exists but can't be read — the occupied check fails.
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let dest = base.join("unreadable");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("x"), "x").unwrap();
    let mut perms = fs::metadata(&dest).unwrap().permissions();
    perms.set_mode(0o111);
    fs::set_permissions(&dest, perms.clone()).unwrap();
    let w = Worktrees::new(&base);
    let err = w.create(&repo, &first, &dest).unwrap_err();
    let mut perms = fs::metadata(&dest).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dest, perms).unwrap();
    assert!(matches!(err, Error::Io { .. }));
}

#[cfg(unix)]
#[test]
fn create_surfaces_persistent_add_failure() {
    // `dest`'s parent isn't writable — `worktree add` fails, the
    // prune-retry fails again, and the real error surfaces.
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    fs::create_dir_all(&base).unwrap();
    let mut perms = fs::metadata(&base).unwrap().permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&base, perms.clone()).unwrap();
    let w = Worktrees::new(&base);
    let err = w.create(&repo, &first, "run-1").unwrap_err();
    let mut perms = fs::metadata(&base).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&base, perms).unwrap();
    assert!(matches!(err, Error::Git { .. }));
}

/// Manifest present but read-only — every `manifest::save` fails at
/// `File::create` while `load` still parses fine.
#[cfg(unix)]
fn readonly_manifest(base: &Path, entries: &serde_json::Value) {
    write_manifest(base, entries);
    let file = base.join(".peregrin-worktrees.json");
    let mut perms = fs::metadata(&file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&file, perms).unwrap();
}

#[cfg(unix)]
#[test]
fn create_surfaces_save_failure_during_adoption() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    readonly_manifest(&base, &serde_json::json!([]));
    // A real but untracked checkout for `create` to adopt.
    let dest = tmp.path().join("stray");
    git(
        &repo,
        &[
            "worktree",
            "add",
            "--detach",
            dest.to_str().unwrap(),
            &first,
        ],
    );
    let w = Worktrees::new(&base);
    let err = w.create(&repo, &first, &dest).unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
}

#[cfg(unix)]
#[test]
fn list_surfaces_save_failure_on_stale_rewrite() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    readonly_manifest(
        &base,
        &serde_json::json!([{
            "path": base.join("gone"),
            "repo": "/repo",
            "sha": "s",
            "kept": false,
        }]),
    );
    let w = Worktrees::new(&base);
    let err = w.list().unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
}

#[cfg(unix)]
#[test]
fn keep_surfaces_save_failure() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    let path = base.join("tracked");
    readonly_manifest(
        &base,
        &serde_json::json!([{
            "path": path.clone(),
            "repo": "/repo",
            "sha": "s",
            "kept": false,
        }]),
    );
    let w = Worktrees::new(&base);
    let err = w.keep(&path).unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
}

#[cfg(unix)]
#[test]
fn destroy_surfaces_save_failure() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("worktrees");
    let path = base.join("gone");
    readonly_manifest(
        &base,
        &serde_json::json!([{
            "path": path.clone(),
            "repo": "/nonexistent-repo",
            "sha": "s",
            "kept": false,
        }]),
    );
    let w = Worktrees::new(&base);
    let err = w.destroy(&path).unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
}

#[test]
fn cleanup_destroys_unkept_leaves_kept_and_orphans() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let keep_me = w.create(&repo, &first, "keep-me").unwrap();
    let doomed = w.create(&repo, &first, "doomed").unwrap();
    let keep_me = w.keep(&keep_me).unwrap();
    assert!(keep_me.kept);
    // An orphan dir survives cleanup — cleanup only owns records.
    let orphan = base.join("orphan");
    fs::create_dir_all(&orphan).unwrap();

    let destroyed = w.cleanup().unwrap();
    assert_eq!(destroyed, vec![doomed.path.clone()]);
    assert!(!doomed.path.exists());
    assert!(keep_me.path.exists());
    assert!(orphan.exists());
    assert!(w.list().unwrap().contains(&Listing::Tracked(keep_me)));

    // Idempotent: nothing left to destroy.
    assert_eq!(w.cleanup().unwrap().len(), 0);
}

#[test]
fn cleanup_reports_failures_and_continues() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let base = tmp.path().join("worktrees");
    let w = Worktrees::new(&base);
    let good = w.create(&repo, &first, "good").unwrap();
    // A record pointing at a plain file in a repo that doesn't exist
    // — destroy has no way to remove it.
    let stuck = base.join("stuck");
    fs::write(&stuck, "not a dir").unwrap();
    write_manifest(
        &base,
        &serde_json::json!([
            {
                "path": good.path.clone(),
                "repo": repo.clone(),
                "sha": first,
                "kept": false,
            },
            {
                "path": stuck.clone(),
                "repo": "/nonexistent-repo",
                "sha": "s",
                "kept": false,
            },
        ]),
    );
    let err = w.cleanup().unwrap_err();
    match &err {
        Error::Cleanup(failures) => assert_eq!(failures, &vec![stuck.clone()]),
        other => panic!("expected Cleanup, got {other:?}"),
    }
    assert!(err.to_string().contains("stuck"));
    let _ = format!("{err:?}");
    // The good worktree was still destroyed first.
    assert!(!good.path.exists());
    assert!(stuck.exists());
}

#[test]
fn spawn_failure_surfaces_program() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let git = SystemGit::with_program("peregrin-git-definitely-missing");
    let _git2 = git.clone();
    let w = Worktrees::with_git(tmp.path().join("worktrees"), git);
    let err = w.create(&repo, &first, "run-1").unwrap_err();
    assert!(matches!(err, Error::Spawn { .. }));
    assert!(err.to_string().contains("peregrin-git-definitely-missing"));
    assert!(err.source().is_some());
    let _ = format!("{err:?}");
}

#[test]
fn system_git_default_and_base_accessor() {
    let tmp = TempDir::new().unwrap();
    let _default = SystemGit::default();
    let w = Worktrees::new(tmp.path().join("base"));
    assert_eq!(w.base(), tmp.path().join("base").as_path());
    // A relative base is made absolute so records survive cwd moves.
    let rel = Worktrees::new("rel-base");
    assert!(rel.base().is_absolute());
    assert!(rel.base().ends_with("rel-base"));
}

#[test]
fn expand_path_handles_tilde_and_plain() {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("/"), PathBuf::from);
    assert_eq!(expand_path(Path::new("~")), home);
    assert_eq!(expand_path(Path::new("~/a/b")), home.join("a/b"));
    assert_eq!(expand_path(Path::new("rel/dir")), PathBuf::from("rel/dir"));
    assert_eq!(
        expand_path(Path::new("/abs/dir")),
        PathBuf::from("/abs/dir")
    );
    // Non-UTF-8 passes through untouched.
    let non_utf8 = Path::new(OsStr::from_bytes(b"\xff\xfe"));
    assert_eq!(expand_path(non_utf8), non_utf8);
}

/// Re-exec guard — `home_dir`'s `$USERPROFILE`-then-`/` fallbacks
/// need HOME unset, and edition-2024 `remove_var` is unsafe; the
/// child process env is the clean seam (same pattern as
/// peregrin-driver-shell's env-mutation tests).
#[test]
fn expand_path_homeless_fallbacks() {
    if std::env::var_os("PERE_HOMELESS").is_some() {
        assert_eq!(expand_path(Path::new("~")), PathBuf::from("/"));
        assert_eq!(expand_path(Path::new("~/x")), PathBuf::from("/x"));
        return;
    }
    let exe = std::env::current_exe().unwrap();
    // `--exact` needs the qualified name — the thread name carries it.
    let status = Command::new(exe)
        .args([std::thread::current().name().unwrap(), "--exact"])
        .env("PERE_HOMELESS", "1")
        .env_remove("HOME")
        .env_remove("USERPROFILE")
        .status()
        .unwrap();
    assert!(status.success());
}
