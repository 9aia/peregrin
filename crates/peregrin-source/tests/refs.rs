//! Ref-grammar matrix — every ADR-0079 form plus the edge cases
//! (`//` vs `://`, scp colons, `#` rules, two-segment splits).
#![allow(
    clippy::unwrap_used,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::path::{Path, PathBuf};

use peregrin_source::{Error, GitRef, HttpRef, Resolved, SourceRef, expand_path, parse_ref};

fn git(input: &str) -> GitRef {
    match parse_ref(input).unwrap() {
        SourceRef::Git(r) => r,
        other => panic!("expected Git, got {other:?}"),
    }
}

fn http(input: &str) -> HttpRef {
    match parse_ref(input).unwrap() {
        SourceRef::Http(r) => r,
        other => panic!("expected Http, got {other:?}"),
    }
}

fn fs(input: &str) -> PathBuf {
    match parse_ref(input).unwrap() {
        SourceRef::Fs(p) => p,
        other => panic!("expected Fs, got {other:?}"),
    }
}

fn err(input: &str) -> String {
    parse_ref(input).unwrap_err().to_string()
}

// --- git: refs -------------------------------------------------------------

#[test]
fn git_shorthand_org_repo() {
    assert_eq!(
        git("git:9aia/peregrin"),
        GitRef {
            url: "https://github.com/9aia/peregrin".into(),
            subdir: None,
            rev: None,
        }
    );
}

#[test]
fn git_shorthand_with_subdir_and_rev() {
    assert_eq!(
        git("git:9aia/peregrin/libraries/ci-gate#v0.2.0"),
        GitRef {
            url: "https://github.com/9aia/peregrin".into(),
            subdir: Some("libraries/ci-gate".into()),
            rev: Some("v0.2.0".into()),
        }
    );
}

#[test]
fn git_host_qualified_with_double_slash_subdir_and_tag() {
    assert_eq!(
        git("git:github.com/9aia/peregrin//libraries/ci-gate#v0.2.0"),
        GitRef {
            url: "https://github.com/9aia/peregrin".into(),
            subdir: Some("libraries/ci-gate".into()),
            rev: Some("v0.2.0".into()),
        }
    );
}

#[test]
fn git_scp_style_with_subdir_and_branch() {
    assert_eq!(
        git("git:git@github.com:9aia/peregrin//libraries/ci-gate#main"),
        GitRef {
            url: "git@github.com:9aia/peregrin".into(),
            subdir: Some("libraries/ci-gate".into()),
            rev: Some("main".into()),
        }
    );
}

#[test]
fn git_full_url_with_nested_group() {
    assert_eq!(
        git("git:https://gitlab.com/grp/sub/repo//dir"),
        GitRef {
            url: "https://gitlab.com/grp/sub/repo".into(),
            subdir: Some("dir".into()),
            rev: None,
        }
    );
    // Full URLs without // are whole repos — no two-segment split.
    assert_eq!(
        git("git:https://gitlab.com/grp/repo"),
        GitRef {
            url: "https://gitlab.com/grp/repo".into(),
            subdir: None,
            rev: None,
        }
    );
}

#[test]
fn git_edge_forms() {
    // ssh scheme + port.
    let r = git("git:ssh://git@host:2222/org/repo//sub#v1");
    assert_eq!(r.url, "ssh://git@host:2222/org/repo");
    assert_eq!(r.subdir.as_deref(), Some("sub"));
    assert_eq!(r.rev.as_deref(), Some("v1"));
    // file:// URL — // after the scheme is the subdir split.
    let r = git("git:file:///srv/repo//bundle/sub#t");
    assert_eq!(r.url, "file:///srv/repo");
    assert_eq!(r.subdir.as_deref(), Some("bundle/sub"));
    // Whole file:// repo, no subdir.
    assert_eq!(git("git:file:///srv/repo").url, "file:///srv/repo");
    // `//.` and bare `//` normalize to no subdir.
    assert_eq!(git("git:a/b//.").subdir, None);
    assert_eq!(git("git:a/b//").subdir, None);
    // Trailing slashes on the subdir strip.
    assert_eq!(git("git:a/b//sub/").subdir.as_deref(), Some("sub"));
    // Two-segment rule with a trailing slash → no subdir.
    let r = git("git:org/repo/");
    assert_eq!(r.url, "https://github.com/org/repo");
    assert_eq!(r.subdir, None);
    // Two-segment '.' subdir → none.
    assert_eq!(git("git:org/repo/.").subdir, None);
    // Local repo paths are whole repos.
    assert_eq!(git("git:/srv/repo").url, "/srv/repo");
    assert_eq!(git("git:./repo").url, "./repo");
    assert_eq!(git("git:~/repo").url, "~/repo");
    // Host-qualified without // splits per the two-segment rule:
    // `github.com/9aia` is the repo, `peregrin` the in-repo path.
    let r = git("git:github.com/9aia/peregrin");
    assert_eq!(r.url, "https://github.com/9aia");
    assert_eq!(r.subdir.as_deref(), Some("peregrin"));
    // scp-style without // is a whole repo (no two-segment split).
    assert_eq!(git("git:git@h:o/r").url, "git@h:o/r");
    assert_eq!(git("git:h:2222/o/r").url, "h:2222/o/r");
    // Single-segment + host-with-dots normalization.
    assert_eq!(git("git:org/repo/sub").url, "https://github.com/org/repo");
    assert_eq!(git("git:repo").url, "https://repo");
}

#[test]
fn git_ref_errors() {
    assert_eq!(err("git:"), "invalid ref: git: ref is empty");
    assert_eq!(err("git:a/b#"), "invalid ref: empty ref after '#'");
    assert_eq!(err("git:#v"), "invalid ref: git: ref missing the repo");
    assert_eq!(err("git:a/b#x#y"), "invalid ref: malformed ref '#x#y'");
    assert_eq!(
        err("git:a/b//../x"),
        "invalid ref: bad in-repo path '//../x'"
    );
    assert_eq!(err("git:a/b///x"), "invalid ref: bad in-repo path '///x'");
    assert_eq!(err("git:a/b/../c"), "invalid ref: bad in-repo path '/../c'");
}

// --- http(s) tarball refs ---------------------------------------------------

#[test]
fn http_tarball_forms() {
    assert_eq!(
        http("https://9aia.com/bundles/ci-gate-0.2.tgz"),
        HttpRef {
            url: "https://9aia.com/bundles/ci-gate-0.2.tgz".into(),
            digest: None,
        }
    );
    assert_eq!(
        http(
            "https://h/x.tar.gz#sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        HttpRef {
            url: "https://h/x.tar.gz".into(),
            digest: Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()),
        }
    );
    // `=` separator + uppercase hex normalizes.
    let r = http(
        "http://127.0.0.1:9/x.tgz#sha256=0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF",
    );
    assert_eq!(
        r.digest.as_deref(),
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
    );
}

#[test]
fn http_ref_errors() {
    assert_eq!(
        err("https://h/x.zip"),
        "invalid ref: http refs must point at a .tgz/.tar.gz tarball: https://h/x.zip"
    );
    assert_eq!(
        err("https://h/x.tgz#v1"),
        "invalid ref: unsupported fragment '#v1' — expected '#sha256:<hex>'"
    );
    assert_eq!(
        err("https://h/x.tgz#sha256:abc"),
        "invalid ref: sha256 digest must be 64 hex chars"
    );
    assert_eq!(
        err(
            "https://h/x.tgz#sha256:zzzz56789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ),
        "invalid ref: sha256 digest must be 64 hex chars"
    );
}

// --- fs refs + unrecognized -------------------------------------------------

#[test]
fn fs_ref_forms() {
    assert_eq!(fs("./ci-gate"), PathBuf::from("./ci-gate"));
    assert_eq!(fs("../x"), PathBuf::from("../x"));
    assert_eq!(fs("/abs/path"), PathBuf::from("/abs/path"));
    assert_eq!(fs("~/x"), PathBuf::from("~/x"));
    assert_eq!(fs("."), PathBuf::from("."));
    assert_eq!(fs(".."), PathBuf::from(".."));
    assert_eq!(fs("~"), PathBuf::from("~"));
    // Bare nested paths parse as fs too.
    assert_eq!(fs("libraries/ci-gate"), PathBuf::from("libraries/ci-gate"));
}

#[test]
fn unrecognized_refs_error() {
    assert_eq!(err(""), "invalid ref: empty ref");
    assert_eq!(err("   "), "invalid ref: empty ref");
    assert_eq!(
        err("ci-gate"),
        "invalid ref: unrecognized ref 'ci-gate' — expected ./path, /path, \
         git:<url>[//subdir][#rev], or an http(s) .tgz/.tar.gz URL"
    );
}

// --- Resolved::content_key + helpers ----------------------------------------

#[test]
fn prefetch_identity_knowable_pre_fetch() {
    let git = Resolved::Git {
        url: "u".into(),
        subdir: None,
        rev: None,
        sha: Some("abc".into()),
    };
    assert_eq!(
        git.prefetch_identity(),
        Some(("git-abc".to_string(), "abc".to_string()))
    );
    let http = Resolved::Http {
        url: "u".into(),
        digest: Some("d".into()),
    };
    assert_eq!(
        http.prefetch_identity(),
        Some(("sha256-d".to_string(), "sha256:d".to_string()))
    );
    for unresolved in [
        Resolved::Fs {
            path: PathBuf::from("x"),
        },
        Resolved::Git {
            url: "u".into(),
            subdir: None,
            rev: Some("main".into()),
            sha: None,
        },
        Resolved::Http {
            url: "u".into(),
            digest: None,
        },
    ] {
        assert_eq!(unresolved.prefetch_identity(), None);
    }
}

#[test]
fn expand_path_and_error_arms() {
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    assert_eq!(expand_path(Path::new("~")), home);
    assert_eq!(expand_path(Path::new("~/x")), home.join("x"));
    assert_eq!(expand_path(Path::new("rel")), PathBuf::from("rel"));

    // Error Display + io() helper.
    let io_err = Error::io("ctx")(std::io::Error::other("boom"));
    assert_eq!(io_err.to_string(), "ctx: boom");
    assert_eq!(Error::Failed("f".into()).to_string(), "f");
}

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

#[cfg(unix)]
#[test]
fn non_utf8_path_passthrough() {
    let odd = PathBuf::from(std::ffi::OsStr::from_bytes(b"\xff"));
    assert_eq!(expand_path(&odd), odd);
}
