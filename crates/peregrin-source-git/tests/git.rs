//! `peregrin-source-git` — real `file://` fixtures for the happy paths
//! (mirroring `peregrin-driver-git`'s suite style: the crate wraps
//! process git, so tests shell out), a canned `Git` for error arms.
#![allow(
    clippy::unwrap_used,
    unknown_lints,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use peregrin_git::{Git, SystemGit};
use peregrin_source::{Resolved, Source, SourceRef, parse_ref};
use peregrin_source_git::GitSource;
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

/// A repo: bundle at root on `main`, a `bundles/gate` subdir, a
/// `v1.0` tag and a `dev` branch — `(repo_dir, head_sha)`.
fn fixture_repo(root: &Path) -> (PathBuf, String) {
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("bundles/gate")).unwrap();
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    fs::write(repo.join("peregrin-bundle.toml"), "name = \"root\"\n").unwrap();
    fs::write(
        repo.join("bundles/gate/peregrin-bundle.yaml"),
        "name: gate\n",
    )
    .unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "one"]);
    git(&repo, &["tag", "-a", "v1.0", "-m", "tag"]);
    fs::write(repo.join("main.yaml"), "name: w\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "two"]);
    git(&repo, &["branch", "dev"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    (repo, head)
}

fn git_ref(input: &str) -> SourceRef {
    parse_ref(input).unwrap()
}

// --- resolve: real git over file:// -------------------------------------------

#[test]
fn resolve_pins_refs_via_ls_remote() {
    let tmp = TempDir::new().unwrap();
    let (repo, head) = fixture_repo(tmp.path());
    let source = GitSource::new();
    let url = format!("file://{}", repo.display());

    // HEAD.
    let resolved = source.resolve(&git_ref(&format!("git:{url}"))).unwrap();
    let Resolved::Git { sha, .. } = &resolved else {
        panic!()
    };
    assert_eq!(sha.as_deref(), Some(head.as_str()));

    // Tag (peeled), branch, full sha, short sha.
    let tag_sha = git(&repo, &["rev-parse", "v1.0^{commit}"]);
    for (rev, want) in [
        ("v1.0", Some(tag_sha.as_str())),
        ("dev", Some(head.as_str())),
        ("main", Some(head.as_str())),
        (head.as_str(), Some(head.as_str())),
        (&head[..9], None),
    ] {
        let resolved = source
            .resolve(&git_ref(&format!("git:{url}#{rev}")))
            .unwrap();
        let Resolved::Git { sha, .. } = &resolved else {
            panic!()
        };
        assert_eq!(sha.as_deref(), want, "rev {rev}");
    }

    // subdir round-trips through resolve.
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}//bundles/gate#main")))
        .unwrap();
    let Resolved::Git { subdir, .. } = &resolved else {
        panic!()
    };
    assert_eq!(subdir.as_deref(), Some("bundles/gate"));
}

#[test]
fn resolve_error_arms() {
    let tmp = TempDir::new().unwrap();
    let (repo, _) = fixture_repo(tmp.path());
    let source = GitSource::new();
    let url = format!("file://{}", repo.display());
    // Missing ref.
    assert!(
        source
            .resolve(&git_ref(&format!("git:{url}#nope")))
            .unwrap_err()
            .to_string()
            .contains("not found")
    );
    // Bad remote.
    assert!(
        source
            .resolve(&git_ref("git:file:///definitely/not/a/repo"))
            .unwrap_err()
            .to_string()
            .contains("ls-remote")
    );
    // Wrong variant.
    assert!(
        source
            .resolve(&SourceRef::Fs(PathBuf::from("x")))
            .unwrap_err()
            .to_string()
            .contains("only resolves git")
    );
}

// --- fetch: real clones --------------------------------------------------------

#[test]
fn fetch_clones_checks_out_and_strips_subdir() {
    let tmp = TempDir::new().unwrap();
    let (repo, head) = fixture_repo(tmp.path());
    let source = GitSource::new();
    let url = format!("file://{}", repo.display());

    // Whole repo at HEAD.
    let resolved = source.resolve(&git_ref(&format!("git:{url}"))).unwrap();
    let dest = tmp.path().join("out");
    let fetched = source.fetch(&resolved, &dest).unwrap();
    assert_eq!(fetched.resolved, head);
    assert_eq!(fetched.content_key, format!("git-{head}"));
    assert!(dest.join("peregrin-bundle.toml").is_file());
    assert!(!dest.join(".clone").exists());
    assert!(!dest.join("bundles/gate").exists() || dest.join("bundles").exists());

    // //subdir — only the subtree lands.
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}//bundles/gate")))
        .unwrap();
    let dest = tmp.path().join("sub");
    source.fetch(&resolved, &dest).unwrap();
    assert!(dest.join("peregrin-bundle.yaml").is_file());
    assert!(!dest.join("bundles").exists());

    // #tag pins the earlier commit — main.yaml doesn't exist there.
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}#v1.0")))
        .unwrap();
    let dest = tmp.path().join("tagged");
    let fetched = source.fetch(&resolved, &dest).unwrap();
    assert_eq!(fetched.resolved, tag_sha(&repo));
    assert!(!dest.join("main.yaml").exists());

    // #short-sha resolves through the clone.
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}#{}", &head[..9])))
        .unwrap();
    let dest = tmp.path().join("short");
    let fetched = source.fetch(&resolved, &dest).unwrap();
    assert_eq!(fetched.resolved, head);
}

fn tag_sha(repo: &Path) -> String {
    git(repo, &["rev-parse", "v1.0^{commit}"])
}

#[test]
fn fetch_error_arms() {
    let tmp = TempDir::new().unwrap();
    let (repo, head) = fixture_repo(tmp.path());
    let source = GitSource::new();
    let url = format!("file://{}", repo.display());

    // Missing subdir.
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}//nope")))
        .unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("o"))
            .unwrap_err()
            .to_string()
            .contains("subdir 'nope' not in")
    );
    // 40-hex sha not in the repo: resolve skips ls-remote, fetch's
    // rev-parse fails.
    let missing_sha = if head.starts_with('0') {
        "1".repeat(40)
    } else {
        "0".repeat(40)
    };
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}#{missing_sha}")))
        .unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("o2"))
            .unwrap_err()
            .to_string()
            .contains("not on")
    );
    // Clone failure.
    let resolved = source
        .resolve(&git_ref(&format!("git:{url}//bundles/gate#main")))
        .unwrap();
    let Resolved::Git {
        subdir, rev, sha, ..
    } = resolved
    else {
        panic!()
    };
    let bad = Resolved::Git {
        url: "file:///definitely/not/a/repo".into(),
        subdir,
        rev,
        sha,
    };
    assert!(
        source
            .fetch(&bad, &tmp.path().join("o3"))
            .unwrap_err()
            .to_string()
            .contains("clone")
    );
    // Wrong variant.
    assert!(
        source
            .fetch(
                &Resolved::Fs {
                    path: PathBuf::from("x")
                },
                &tmp.path().join("o4")
            )
            .unwrap_err()
            .to_string()
            .contains("only fetches git")
    );
    // with_git / Default constructors.
    let _ = GitSource::with_git(SystemGit::new());
    let _ = GitSource::default();
}

// --- stub git for the remaining arms --------------------------------------------

/// Canned `Git` — fails when `fail_on` appears in the args, else
/// returns `stdout` (or a canned ls-remote for that call).
struct StubGit {
    stdout: String,
    fail_on: Option<&'static str>,
}

impl Git for StubGit {
    fn run(&self, _dir: &Path, args: &[&OsStr]) -> Result<String, peregrin_git::Error> {
        if self
            .fail_on
            .is_some_and(|needle| args.iter().any(|a| a.to_str() == Some(needle)))
        {
            return Err(peregrin_git::Error::Git {
                dir: PathBuf::new(),
                args: args
                    .iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
                status: "1".into(),
                stderr: "stub".into(),
            });
        }
        Ok(self.stdout.clone())
    }
}

#[test]
fn ls_remote_parse_and_headless_remote() {
    let stub = StubGit {
        stdout: "abc123\tHEAD\nsha1\trefs/heads/main\nsha2\trefs/tags/v1\nsha3\trefs/tags/v1^{}\nskip-me\n".into(),
        fail_on: None,
    };
    let source = GitSource::with_git(stub);
    // Tag prefers the peeled ^{} line.
    let resolved = source.resolve(&git_ref("git:x/y#v1")).unwrap();
    let Resolved::Git { sha, .. } = resolved else {
        panic!()
    };
    assert_eq!(sha.as_deref(), Some("sha3"));
    // A remote advertising no HEAD.
    let headless = GitSource::with_git(StubGit {
        stdout: "sha\trefs/heads/x\n".into(),
        fail_on: None,
    });
    assert!(
        headless
            .resolve(&git_ref("git:x/y"))
            .unwrap_err()
            .to_string()
            .contains("no HEAD")
    );
    // rev-parse failure inside fetch surfaces the 'not on' arm.
    let failing = GitSource::with_git(StubGit {
        stdout: String::new(),
        fail_on: Some("rev-parse"),
    });
    let resolved = Resolved::Git {
        url: "u".into(),
        subdir: None,
        rev: Some("main".into()),
        sha: Some("s".into()),
    };
    assert!(
        failing
            .fetch(
                &resolved,
                &tempfile::TempDir::new().unwrap().path().join("d")
            )
            .unwrap_err()
            .to_string()
            .contains("not on")
    );
    // checkout failure.
    let failing = GitSource::with_git(StubGit {
        stdout: "sha".into(),
        fail_on: Some("checkout"),
    });
    assert!(
        failing
            .fetch(
                &resolved,
                &tempfile::TempDir::new().unwrap().path().join("d2")
            )
            .is_err()
    );
    // ~ clone paths expand.
    let expanded = source.resolve(&git_ref("git:~/repo#main")).unwrap();
    let Resolved::Git { url, .. } = expanded else {
        panic!()
    };
    assert!(!url.starts_with('~'), "{url}");
    let _ = OsString::new();
}

#[test]
fn fetch_copy_failure_folds_bundle_error() {
    // dest_dir has a dir where the payload has a same-name file →
    // copy_tree fails through `be`.
    let tmp = TempDir::new().unwrap();
    let (repo, _) = fixture_repo(tmp.path());
    let source = GitSource::new();
    let url = format!("file://{}", repo.display());
    let resolved = source.resolve(&git_ref(&format!("git:{url}"))).unwrap();
    let dest = tmp.path().join("blocked");
    fs::create_dir_all(dest.join("peregrin-bundle.toml")).unwrap();
    assert!(source.fetch(&resolved, &dest).is_err());
}
