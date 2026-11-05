//! `peregrin install` & friends — e2e over `DistEnv` sandboxes: fs
//! dirs, `git:` file:// fixtures (branch/tag/sha/`//subdir`), an http
//! tarball via `TcpListener`, include composition, and every verb's
//! error arms.
#![allow(
    clippy::unwrap_used,
    unknown_lints,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use peregrin_bundle::{FetchCache, Store};
use peregrin_cli::distribution::{self, DistEnv};
use tempfile::TempDir;

// --- fixtures -------------------------------------------------------------------

/// A bundle dir on disk — manifest + a campaign doc.
fn bundle_dir(root: &Path, name: &str, version: Option<&str>, extra: &[(&str, &str)]) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    let version_line = version.map_or(String::new(), |v| format!("version: {v}\n"));
    fs::write(
        dir.join("peregrin-bundle.yaml"),
        format!("name: {name}\n{version_line}"),
    )
    .unwrap();
    for (rel, body) in extra {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    dir
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A git repo holding `bundles/gate` on main + tag `v1.0` — `(repo, head)`.
fn git_repo(root: &Path) -> (PathBuf, String) {
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("bundles/gate")).unwrap();
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.email", "t@e.c"]);
    git(&repo, &["config", "user.name", "t"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    fs::write(
        repo.join("bundles/gate/peregrin-bundle.toml"),
        "name = \"gate\"\n",
    )
    .unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "one"]);
    git(&repo, &["tag", "v1.0"]);
    fs::write(repo.join("bundles/gate/main.yaml"), "name: w\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "two"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    (repo, head)
}

fn serve_once(status: &'static str, body: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 8192];
        let _ = stream.read(&mut buf);
        let head = format!(
            "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(&body).unwrap();
    });
    format!("http://127.0.0.1:{port}/b.tgz")
}

// --- fs installs + the verb surface ----------------------------------------------

#[test]
fn fs_install_then_list_info_verify_remove() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    let bundle = bundle_dir(
        tmp.path(),
        "gate",
        Some("0.2.0"),
        &[("main.yaml", "name: w\n")],
    );
    let entry = distribution::install(&env, bundle.to_str().unwrap()).unwrap();
    assert_eq!(entry.label(), "gate@0.2.0");
    assert_eq!(entry.record.source_ref, bundle.to_str().unwrap());

    // Repeating hits the fetch cache — same digest, still one entry.
    let again = distribution::install(&env, bundle.to_str().unwrap()).unwrap();
    assert_eq!(again.record.digest, entry.record.digest);
    assert_eq!(Store::new(&env.store_root).entries().unwrap().len(), 1);
    assert_eq!(FetchCache::new(&env.cache_root).entries().unwrap().len(), 1);

    // list / info / verify.
    let lines = distribution::list(&env).unwrap();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("gate@0.2.0"));
    let info = distribution::info(&env, "gate").unwrap();
    assert!(info.iter().any(|l| l.contains("source:")));
    assert!(info.iter().any(|l| l.contains("size:")));
    assert!(distribution::verify(&env, "gate").unwrap().ok);

    // Tamper → verify errors.
    fs::write(entry.dir.join("main.yaml"), "tampered\n").unwrap();
    let err = distribution::verify(&env, "gate").unwrap_err().to_string();
    assert!(err.contains("digest mismatch"), "{err}");
    fs::write(entry.dir.join("main.yaml"), "name: w\n").unwrap();
    assert!(distribution::verify(&env, "gate").unwrap().ok);

    // remove drops the entry, cache untouched.
    distribution::remove(&env, "gate").unwrap();
    assert!(Store::new(&env.store_root).entries().unwrap().is_empty());
    assert_eq!(FetchCache::new(&env.cache_root).entries().unwrap().len(), 1);
    assert!(
        distribution::remove(&env, "gate")
            .unwrap_err()
            .to_string()
            .contains("not installed")
    );

    // Empty store lists clean.
    let lines = distribution::list(&env).unwrap();
    assert!(lines.is_empty());
}

#[test]
fn install_error_arms() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    // Bad grammar.
    assert!(distribution::install(&env, "bare-name").is_err());
    // Missing path.
    assert!(
        distribution::install(&env, "./nope")
            .unwrap_err()
            .to_string()
            .contains("no such path")
    );
    // No manifest under the payload.
    let bare = tmp.path().join("bare");
    fs::create_dir_all(&bare).unwrap();
    fs::write(bare.join("x"), "x").unwrap();
    assert!(
        distribution::install(&env, bare.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("no bundle manifest")
    );
    // Manifest parse/validation failure.
    let bad = tmp.path().join("bad");
    fs::create_dir_all(&bad).unwrap();
    fs::write(bad.join("peregrin-bundle.toml"), "name = \"a/b\"\n").unwrap();
    assert!(distribution::install(&env, bad.to_str().unwrap()).is_err());
    // Include cycle.
    let cyc = tmp.path().join("cyc");
    fs::create_dir_all(&cyc).unwrap();
    fs::write(
        cyc.join("peregrin-bundle.toml"),
        format!("name = \"cyc\"\ninclude = [\"{}\"]\n", cyc.display()),
    )
    .unwrap();
    assert!(
        distribution::install(&env, cyc.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("include cycle")
    );
}

// --- include composition ----------------------------------------------------------

#[test]
fn include_composes_from_store_names_and_refs() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    // Base bundle installed first, by store name.
    let base = bundle_dir(
        tmp.path(),
        "base-lib",
        None,
        &[(
            "main.yaml",
            "name: b\nnodes:\n  - id: gate\n    driver: shell\n  - id: extra\n    driver: shell\n",
        )],
    );
    distribution::install(&env, base.to_str().unwrap()).unwrap();
    // Overlay includes it by name + a ref (itself through the store).
    let over = tmp.path().join("over");
    fs::create_dir_all(&over).unwrap();
    fs::write(
        over.join("peregrin-bundle.yaml"),
        format!(
            "name: over\nversion: 1.0\ninclude: [base-lib, \"{}\"]\n",
            base.display()
        ),
    )
    .unwrap();
    fs::write(
        over.join("main.yaml"),
        "nodes:\n  - id: gate\n    driver: agent\n",
    )
    .unwrap();
    let entry = distribution::install(&env, over.to_str().unwrap()).unwrap();
    let merged: serde_json::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(entry.dir.join("main.yaml")).unwrap()).unwrap();
    assert_eq!(
        merged["nodes"],
        serde_json::json!([{"id": "gate", "driver": "agent"}, {"id": "extra", "driver": "shell"}])
    );
    let info = distribution::info(&env, "over").unwrap();
    assert!(info.iter().any(|l| l.contains("include:")));
    // A missing store-name include errors.
    let broken = tmp.path().join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(
        broken.join("peregrin-bundle.yaml"),
        "name: broken\ninclude: [ghost]\n",
    )
    .unwrap();
    assert!(
        distribution::install(&env, broken.to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("include 'ghost'")
    );
}

// --- git installs + update/pin -----------------------------------------------------

#[test]
fn git_install_update_and_pin_over_file_urls() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    let (repo, head) = git_repo(tmp.path());
    let url = format!("file://{}", repo.display());

    // //subdir + #tag pin — main.yaml isn't in the tag.
    let entry = distribution::install(&env, &format!("git:{url}//bundles/gate#v1.0")).unwrap();
    assert_eq!(
        entry.label(),
        format!("gate@{}", &entry.record.resolved_sha[..12])
    );
    assert!(!entry.dir.join("main.yaml").exists());
    let tagged_sha = entry.record.resolved_sha.clone();
    // Different shas → different labels → bare 'gate' would be
    // ambiguous; drop the tag install before the branch one.
    distribution::remove(&env, &entry.label()).unwrap();

    // A moving branch ref: install at head…
    let branch_ref = format!("git:{url}//bundles/gate#main");
    let entry = distribution::install(&env, &branch_ref).unwrap();
    assert_eq!(entry.record.resolved_sha, head);
    assert!(entry.dir.join("main.yaml").exists());
    // …update re-resolves; nothing moved → already current.
    let lines = distribution::update(&env, Some("gate"), false).unwrap();
    assert!(lines[0].contains("already current"), "{lines:?}");

    // Advance main, update → new sha lands (same label dir replaced).
    fs::write(repo.join("bundles/gate/new.yaml"), "x: 1\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "three"]);
    let new_head = git(&repo, &["rev-parse", "HEAD"]);
    let lines = distribution::update(&env, Some("gate"), false).unwrap();
    assert!(lines[0].contains(&new_head[..12]), "{lines:?}");
    let entry = Store::new(&env.store_root).find("gate").unwrap();
    assert_eq!(entry.record.resolved_sha, new_head);

    // --all covers every entry.
    let lines = distribution::update(&env, None, true).unwrap();
    assert_eq!(lines.len(), 1);

    // pin back to the tag → digest differs → retarget + old dir stays
    // only if the label differs; here the sha label changed so the
    // stale dir is dropped.
    let label_before = entry.label();
    let line = distribution::pin(&env, "gate", &format!("git:{url}//bundles/gate#v1.0")).unwrap();
    assert!(line.contains("pinned"), "{line}");
    let entry = Store::new(&env.store_root).find("gate").unwrap();
    assert_eq!(entry.record.resolved_sha, tagged_sha);
    assert_ne!(entry.label(), label_before);
    assert!(!entry.dir.join("main.yaml").exists());
    assert_eq!(Store::new(&env.store_root).entries().unwrap().len(), 1);
    // pin unchanged → the digest-equal arm.
    let line = distribution::pin(&env, "gate", &format!("git:{url}//bundles/gate#v1.0")).unwrap();
    assert!(line.contains("unchanged"), "{line}");

    // update with neither name nor --all.
    assert!(distribution::update(&env, None, false).is_err());
    // update a missing bundle.
    assert!(distribution::update(&env, Some("ghost"), false).is_err());
    // A full-sha pin.
    let line =
        distribution::pin(&env, "gate", &format!("git:{url}//bundles/gate#{new_head}")).unwrap();
    assert!(line.contains(&new_head[..12]), "{line}");
}

// --- http installs -----------------------------------------------------------------

#[test]
fn http_tarball_install_and_pinned_cache_hit() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    let bundle = bundle_dir(tmp.path(), "tball", Some("9"), &[]);
    let tgz = tmp.path().join("tball.tgz");
    distribution::bundle_pack(&bundle, Some(&tgz)).unwrap();
    let bytes = fs::read(&tgz).unwrap();
    let digest = peregrin_bundle::sha256_hex(&bytes);

    let url = serve_once("200 OK", bytes.clone());
    let entry = distribution::install(&env, &format!("{url}#sha256:{digest}")).unwrap();
    assert_eq!(entry.label(), "tball@9");
    assert_eq!(entry.record.resolved_sha, format!("sha256:{digest}"));

    // Second install of the pinned URL: the content key is known
    // pre-fetch → cache hit, no second request (the listener is gone —
    // a fetch would fail the test).
    let url = format!("{url}#sha256:{digest}");
    let entry = distribution::install(&env, &url).unwrap();
    assert_eq!(entry.record.resolved_sha, format!("sha256:{digest}"));

    // Unpinned tarball — digested on the way in.
    let url = serve_once("200 OK", bytes);
    let entry = distribution::install(&env, &url).unwrap();
    assert!(entry.record.resolved_sha.starts_with("sha256:"));
}

// --- cache + pack verbs ------------------------------------------------------------

#[test]
fn cache_clean_and_bundle_pack() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    let bundle = bundle_dir(tmp.path(), "c", None, &[]);
    distribution::install(&env, bundle.to_str().unwrap()).unwrap();
    // clean --days 0 evicts everything fresh; --all agrees.
    assert_eq!(distribution::cache_clean(&env, false, Some(0)).unwrap(), 1);
    distribution::install(&env, bundle.to_str().unwrap()).unwrap();
    assert_eq!(distribution::cache_clean(&env, true, None).unwrap(), 1);
    // Empty cache still reports.
    assert_eq!(distribution::cache_clean(&env, false, None).unwrap(), 0);

    // pack → name-version.tgz default name, plus explicit --out.
    let bundle = bundle_dir(tmp.path(), "packed", Some("2.1"), &[("f", "x")]);
    let cwd = std::env::current_dir().unwrap();
    let out = distribution::bundle_pack(&bundle, None).unwrap();
    assert_eq!(out, PathBuf::from("packed-2.1.tgz"));
    assert!(cwd.join(&out).is_file());
    fs::remove_file(&out).unwrap();
    let explicit = tmp.path().join("custom.tar.gz");
    assert_eq!(
        distribution::bundle_pack(&bundle, Some(&explicit)).unwrap(),
        explicit
    );
    // Unversioned default + missing-manifest error.
    let bundle = bundle_dir(tmp.path(), "nov", None, &[]);
    let out = distribution::bundle_pack(&bundle, None).unwrap();
    assert_eq!(out, PathBuf::from("nov.tgz"));
    assert!(cwd.join(&out).is_file());
    fs::remove_file(&out).unwrap();
    assert!(distribution::bundle_pack(&tmp.path().join("nope"), None).is_err());
}

// --- DistEnv roots -------------------------------------------------------------------

#[test]
fn dist_env_defaults_and_under() {
    let tmp = TempDir::new().unwrap();
    let env = DistEnv::under(tmp.path());
    assert_eq!(env.store_root, tmp.path().join("store"));
    assert_eq!(env.cache_root, tmp.path().join("fetch"));
    let default = DistEnv::default();
    assert!(default.store_root.ends_with("peregrin/store"));
    assert!(default.cache_root.ends_with("fetch"));
}
