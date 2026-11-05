//! `peregrin-source-fs` — dir copy + tarball unpack e2e over tempdirs.
#![allow(
    clippy::unwrap_used,
    unknown_lints,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::fs;
use std::path::PathBuf;

use peregrin_bundle::pack_tgz;
use peregrin_source::{Resolved, Source, SourceRef, parse_ref};
use peregrin_source_fs::FsSource;
use tempfile::TempDir;

fn fs_ref(path: &std::path::Path) -> SourceRef {
    SourceRef::Fs(path.to_path_buf())
}

#[test]
fn resolve_and_fetch_a_dir() {
    let tmp = TempDir::new().unwrap();
    let bundle = tmp.path().join("b");
    fs::create_dir_all(bundle.join("sub")).unwrap();
    fs::write(bundle.join("peregrin-bundle.yaml"), "name: b\n").unwrap();
    fs::write(bundle.join("sub/x"), "x").unwrap();
    fs::create_dir_all(bundle.join(".git")).unwrap();
    fs::write(bundle.join(".git/hidden"), "h").unwrap();

    let source = FsSource::new();
    let resolved = source.resolve(&fs_ref(&bundle)).unwrap();
    assert_eq!(
        resolved,
        Resolved::Fs {
            path: bundle.clone()
        }
    );
    let dest = tmp.path().join("dest");
    let fetched = source.fetch(&resolved, &dest).unwrap();
    assert_eq!(fetched.dir, dest);
    assert!(fetched.resolved.starts_with("sha256:"));
    assert!(fetched.content_key.starts_with("sha256-"));
    assert!(dest.join("peregrin-bundle.yaml").is_file());
    assert!(dest.join("sub/x").is_file());
    assert!(!dest.join(".git").exists());
    // A repeat fetch of identical bytes gives the same key.
    let dest2 = tmp.path().join("dest2");
    assert_eq!(
        source.fetch(&resolved, &dest2).unwrap().content_key,
        fetched.content_key
    );
}

#[test]
fn fetch_unpacks_tarbals_and_rejects_other_files() {
    let tmp = TempDir::new().unwrap();
    let bundle = tmp.path().join("b");
    fs::create_dir_all(&bundle).unwrap();
    fs::write(bundle.join("peregrin-bundle.yaml"), "name: b\n").unwrap();
    let source = FsSource::new();
    for name in ["b.tgz", "b.tar.gz"] {
        let tgz = tmp.path().join(name);
        pack_tgz(&bundle, &tgz).unwrap();
        let resolved = source.resolve(&fs_ref(&tgz)).unwrap();
        let dest = tmp.path().join(format!("out-{name}"));
        let fetched = source.fetch(&resolved, &dest).unwrap();
        assert!(dest.join("peregrin-bundle.yaml").is_file());
        assert!(fetched.resolved.starts_with("sha256:"));
    }
    // Plain .tar too.
    let tar_path = tmp.path().join("b.tar");
    {
        let mut builder = tar::Builder::new(fs::File::create(&tar_path).unwrap());
        builder
            .append_path_with_name(bundle.join("peregrin-bundle.yaml"), "peregrin-bundle.yaml")
            .unwrap();
    }
    let resolved = source.resolve(&fs_ref(&tar_path)).unwrap();
    let dest = tmp.path().join("out-tar");
    source.fetch(&resolved, &dest).unwrap();
    assert!(dest.join("peregrin-bundle.yaml").is_file());
    // Unsupported file types error.
    let odd = tmp.path().join("b.zip");
    fs::write(&odd, "x").unwrap();
    let resolved = source.resolve(&fs_ref(&odd)).unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("z"))
            .unwrap_err()
            .to_string()
            .contains("unsupported bundle file")
    );
}

#[test]
fn resolve_and_fetch_error_arms() {
    let tmp = TempDir::new().unwrap();
    let source = FsSource::new();
    // Wrong ref variants.
    let git_ref = parse_ref("git:o/r").unwrap();
    assert!(
        source
            .resolve(&git_ref)
            .unwrap_err()
            .to_string()
            .contains("only resolves fs")
    );
    let http_resolved = Resolved::Http {
        url: "u".into(),
        digest: None,
    };
    assert!(
        source
            .fetch(&http_resolved, &tmp.path().join("d"))
            .unwrap_err()
            .to_string()
            .contains("only fetches fs")
    );
    // Missing path.
    let missing = SourceRef::Fs(tmp.path().join("nope"));
    assert!(
        source
            .resolve(&missing)
            .unwrap_err()
            .to_string()
            .contains("no such path")
    );
    // Default constructor.
    let _ = FsSource;
    let _default = FsSource::default();
    let _ = PathBuf::new(); // keep imports honest
}

#[test]
fn tarball_unpack_failure_folds_bundle_error() {
    // A .tgz whose bytes aren't gzip → peregrin-bundle's error maps
    // through `be`.
    let tmp = TempDir::new().unwrap();
    let bad = tmp.path().join("bad.tgz");
    fs::write(&bad, "not gzip").unwrap();
    let source = FsSource::new();
    let resolved = source.resolve(&fs_ref(&bad)).unwrap();
    assert!(source.fetch(&resolved, &tmp.path().join("o")).is_err());
}
