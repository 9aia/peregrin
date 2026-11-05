//! peregrin-bundle — manifest, digest, archive, overlay, store and
//! cache coverage over tempdir fixtures.
#![allow(
    clippy::unwrap_used,
    unknown_lints,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use peregrin_bundle::{
    Error, FetchCache, Manifest, Store, StoreRecord, copy_tree, dir_digest, dir_size, file_digest,
    overlay, pack_tgz, payload_root, sha256_hex, untar, untar_gz,
};
use tempfile::TempDir;

fn fixture(dir: &Path, name: &str, version: Option<&str>) {
    fs::create_dir_all(dir).unwrap();
    let version = version.map_or(String::new(), |v| format!("version = \"{v}\"\n"));
    fs::write(
        dir.join("peregrin-bundle.toml"),
        format!("name = \"{name}\"\n{version}"),
    )
    .unwrap();
    fs::write(dir.join("main.yaml"), "name: w\nnodes: []\n").unwrap();
}

fn record(name: &str, version: Option<&str>) -> StoreRecord {
    StoreRecord {
        name: name.into(),
        version: version.map(str::to_string),
        source_ref: "./x".into(),
        resolved_sha: "deadbeef".into(),
        digest: String::new(),
        installed_at: 0,
    }
}

// --- manifest ----------------------------------------------------------------

#[test]
fn manifest_loads_all_syntaxes_with_yaml_first() {
    let tmp = TempDir::new().unwrap();
    // yaml wins over yml over toml.
    for (file, body) in [
        ("peregrin-bundle.toml", "name = \"toml-m\"\n"),
        ("peregrin-bundle.yml", "name: yml-m\n"),
        ("peregrin-bundle.yaml", "name: yaml-m\n"),
    ] {
        fs::write(tmp.path().join(file), body).unwrap();
    }
    assert_eq!(Manifest::load(tmp.path()).unwrap().name, "yaml-m");
    fs::remove_file(tmp.path().join("peregrin-bundle.yaml")).unwrap();
    assert_eq!(Manifest::load(tmp.path()).unwrap().name, "yml-m");
    fs::remove_file(tmp.path().join("peregrin-bundle.yml")).unwrap();
    let m = Manifest::load(tmp.path()).unwrap();
    assert_eq!(m.name, "toml-m");
    assert_eq!(
        Manifest::path_in(tmp.path()).unwrap(),
        tmp.path().join("peregrin-bundle.toml")
    );
    // include + description round-trip.
    fs::write(
        tmp.path().join("peregrin-bundle.yaml"),
        "name: n\nversion: 1.0.0\ndescription: d\ninclude: [a, git:o/r]\n",
    )
    .unwrap();
    let m = Manifest::load(tmp.path()).unwrap();
    assert_eq!(m.include, ["a", "git:o/r"]);
    assert_eq!(m.description.as_deref(), Some("d"));
}

#[test]
fn manifest_errors() {
    let tmp = TempDir::new().unwrap();
    assert!(Manifest::path_in(tmp.path()).is_none());
    assert!(
        Manifest::load(tmp.path())
            .unwrap_err()
            .to_string()
            .contains("no peregrin-bundle")
    );
    // Parse failures per syntax.
    fs::write(tmp.path().join("peregrin-bundle.yaml"), "{").unwrap();
    assert!(
        Manifest::load(tmp.path())
            .unwrap_err()
            .to_string()
            .contains("parse")
    );
    fs::write(tmp.path().join("peregrin-bundle.toml"), "name = 3\n").unwrap();
    fs::remove_file(tmp.path().join("peregrin-bundle.yaml")).unwrap();
    assert!(
        Manifest::load(tmp.path())
            .unwrap_err()
            .to_string()
            .contains("parse")
    );
    // Validation matrix.
    for (name, version, include, want_err) in [
        ("", None, vec![], "invalid bundle name ''"),
        ("a/b", None, vec![], "invalid bundle name 'a/b'"),
        ("a@b", None, vec![], "invalid bundle name 'a@b'"),
        ("-x", None, vec![], "invalid bundle name '-x'"),
        ("ok", Some(""), vec![], "invalid bundle version ''"),
        ("ok", Some("a/b"), vec![], "invalid bundle version 'a/b'"),
        ("ok", None, vec![" ".into()], "manifest include entries"),
    ] {
        let m = Manifest {
            name: name.into(),
            version: version.map(str::to_string),
            include,
            ..Manifest::default()
        };
        assert!(
            m.validate().unwrap_err().to_string().contains(want_err),
            "{name}/{version:?}"
        );
    }
    // load_at: read + dispatch-by-extension errors.
    assert!(Manifest::load_at(&tmp.path().join("missing.yaml")).is_err());
    let odd = tmp.path().join("manifest.txt");
    fs::write(&odd, "name = \"txt\"\n").unwrap();
    assert_eq!(Manifest::load_at(&odd).unwrap().name, "txt");
}

// --- digest -------------------------------------------------------------------

#[test]
fn digests_cover_the_payload() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("p");
    fixture(&dir, "d", None);
    fs::create_dir_all(dir.join(".git/objects")).unwrap();
    fs::write(dir.join(".git/HEAD"), "ref").unwrap();
    fs::write(dir.join(".peregrin-store.json"), "{}").unwrap();
    let first = dir_digest(&dir).unwrap();
    // .git + the record file don't count.
    fs::write(dir.join(".git/x"), "more").unwrap();
    fs::write(dir.join(".peregrin-store.json"), "{\"a\":1}").unwrap();
    assert_eq!(dir_digest(&dir).unwrap(), first);
    // A payload change moves the digest.
    fs::write(dir.join("extra.txt"), "x").unwrap();
    assert_ne!(dir_digest(&dir).unwrap(), first);
    assert_eq!(sha256_hex(b"abc").len(), 64);
    assert_eq!(file_digest(&dir.join("extra.txt")).unwrap().len(), 64);
    assert!(dir_size(&dir).unwrap() > 0);
    // Error arms: missing dir / unreadable file.
    assert!(dir_digest(&tmp.path().join("nope")).is_err());
    assert!(file_digest(&tmp.path().join("nope")).is_err());
    // A file named like the record file only skips at the root.
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("sub/.peregrin-store.json"), "x").unwrap();
    let _ = dir_digest(&dir).unwrap();
}

// --- archive -------------------------------------------------------------------

/// A hand-rolled tar header — the only way to get hostile paths into
/// an archive (`tar::Builder` sanitizes).
fn raw_tar(entries: &[(&str, u8, &[u8], Option<&str>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, typeflag, data, linkname) in entries {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        h[100..108].copy_from_slice(b"0000755\0");
        h[108..116].copy_from_slice(b"0000000\0");
        h[116..124].copy_from_slice(b"0000000\0");
        h[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
        h[148..156].copy_from_slice(b"        ");
        h[156] = *typeflag;
        if let Some(link) = linkname {
            h[157..157 + link.len()].copy_from_slice(link.as_bytes());
        }
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        let sum: u32 = h.iter().map(|b| u32::from(*b)).sum();
        h[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        out.extend_from_slice(&h);
        out.extend_from_slice(data);
        let pad = (512 - data.len() % 512) % 512;
        out.extend_from_slice(&vec![0u8; pad]);
    }
    out.extend_from_slice(&[0u8; 1024]);
    out
}

#[test]
fn pack_then_untar_round_trips_with_modes() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    fixture(&src, "p", Some("1.0"));
    fs::create_dir_all(src.join("bin")).unwrap();
    fs::write(src.join("bin/run.sh"), "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(src.join("bin/run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let tgz = pack_tgz(&src, &tmp.path().join("p.tgz")).unwrap();
    let dest = tmp.path().join("out");
    untar_gz(fs::File::open(&tgz).unwrap(), &dest).unwrap();
    assert!(dest.join("peregrin-bundle.toml").is_file());
    assert_eq!(
        fs::read_to_string(dest.join("bin/run.sh")).unwrap(),
        "#!/bin/sh\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(dest.join("bin/run.sh"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0o111
        );
    }
    // pack errors: missing src, unwritable out.
    assert!(pack_tgz(&tmp.path().join("nope"), &tgz).is_err());
    assert!(pack_tgz(&src, &tmp.path().join("no/dir/p.tgz")).is_err());
}

#[test]
fn untar_handles_dirs_symlinks_and_rejects_hostiles() {
    let tmp = TempDir::new().unwrap();
    // '.' entry, explicit dir, file, valid symlink.
    let archive = raw_tar(&[
        ("./", b'5', b"", None),
        ("d/", b'5', b"", None),
        ("d/f", b'0', b"x", None),
        ("l", b'2', b"", Some("d/f")),
    ]);
    let dest = tmp.path().join("ok");
    untar(archive.as_slice(), &dest).unwrap();
    assert_eq!(fs::read_to_string(dest.join("d/f")).unwrap(), "x");
    #[cfg(unix)]
    assert_eq!(fs::read_link(dest.join("l")).unwrap(), PathBuf::from("d/f"));
    // Hostile paths error.
    for (name, typeflag, link) in [
        ("../evil", b'0', None),
        ("/abs", b'0', None),
        ("link", b'2', Some("/etc/passwd")),
        ("link2", b'2', Some("../escape")),
    ] {
        let archive = raw_tar(&[(name, typeflag, b"x", link)]);
        assert!(
            untar(archive.as_slice(), &tmp.path().join("h")).is_err(),
            "{name}"
        );
    }
    // Unsupported entry type.
    let archive = raw_tar(&[("fifo", b'6', b"", None)]);
    assert!(
        untar(archive.as_slice(), &tmp.path().join("u"))
            .unwrap_err()
            .to_string()
            .contains("unsupported")
    );
    // Corrupt archive errors on the entry read.
    assert!(untar(&b"not a tar"[..], &tmp.path().join("c")).is_err());
    assert!(untar_gz(&b"not gzip"[..], &tmp.path().join("g")).is_err());
    // Symlink without a target.
    let archive = raw_tar(&[("dangling", b'2', b"", None)]);
    assert!(untar(archive.as_slice(), &tmp.path().join("s")).is_err());
}

// --- overlay + payload_root ----------------------------------------------------

#[test]
fn overlay_merges_docs_and_dedupes_nodes() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().join("base");
    fs::create_dir_all(&base).unwrap();
    fs::write(
        base.join("main.yaml"),
        "name: base\nnodes:\n  - id: gate\n    driver: shell\n  - id: extra\n    driver: shell\nconfig:\n  region: us\n",
    )
    .unwrap();
    fs::write(base.join("lib.node.yml"), "id: lib\n").unwrap();
    let local = tmp.path().join("local");
    fs::create_dir_all(&local).unwrap();
    fs::write(
        local.join("main.yaml"),
        "nodes:\n  - id: gate\n    driver: agent\nconfig:\n  timeout: 5\n",
    )
    .unwrap();
    fs::write(local.join("own.txt"), "local file\n").unwrap();
    let dest = tmp.path().join("out");
    overlay(&[base.as_path()], &local, &dest).unwrap();
    let merged: serde_json::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(dest.join("main.yaml")).unwrap()).unwrap();
    // `gate` kept once, from local; `extra` from base; config merges.
    assert_eq!(
        merged["nodes"],
        serde_json::json!([{"id": "gate", "driver": "agent"}, {"id": "extra", "driver": "shell"}])
    );
    assert_eq!(
        merged["config"],
        serde_json::json!({"timeout": 5, "region": "us"})
    );
    assert!(dest.join("lib.node.yml").is_file());
    assert_eq!(
        fs::read_to_string(dest.join("own.txt")).unwrap(),
        "local file\n"
    );
}

#[test]
fn overlay_priorities_and_non_doc_conflicts() {
    let tmp = TempDir::new().unwrap();
    let mk = |dir: &str, files: &[(&str, &str)]| {
        let d = tmp.path().join(dir);
        fs::create_dir_all(&d).unwrap();
        for (rel, body) in files {
            fs::create_dir_all(d.join(rel).parent().unwrap()).unwrap();
            fs::write(d.join(rel), body).unwrap();
        }
        d
    };
    let early = mk(
        "early",
        &[("main.yaml", "v: early\n"), ("plain.txt", "early\n")],
    );
    let late = mk(
        "late",
        &[
            ("main.yaml", "v: late\n"),
            ("plain.txt", "late\n"),
            ("late-only", "x\n"),
        ],
    );
    let local = mk(
        "local",
        &[
            ("main.yaml", "not a doc: [unclosed\n"),
            ("plain.txt", "local\n"),
            ("data.json", "{\"n\": 1}"),
        ],
    );
    let dest = tmp.path().join("out");
    overlay(&[early.as_path(), late.as_path()], &local, &dest).unwrap();
    // Local isn't parseable → byte win over the merged base doc.
    assert_eq!(
        fs::read_to_string(dest.join("main.yaml")).unwrap(),
        "not a doc: [unclosed\n"
    );
    assert_eq!(
        fs::read_to_string(dest.join("plain.txt")).unwrap(),
        "local\n"
    );
    assert!(dest.join("late-only").is_file());
    // And between bases: earlier include wins on doc merge…
    let dest2 = tmp.path().join("out2");
    let quiet = mk("quiet", &[]);
    overlay(&[early.as_path(), late.as_path()], &quiet, &dest2).unwrap();
    let merged: serde_json::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(dest2.join("main.yaml")).unwrap()).unwrap();
    assert_eq!(merged["v"], "early");
    // …but whole-file conflicts fall to the earlier base too.
    assert_eq!(
        fs::read_to_string(dest2.join("plain.txt")).unwrap(),
        "early\n"
    );
    // JSON docs merge as well (local json over an unparseable base json).
    let base = mk("base2", &[("data.json", "not json")]);
    let dest3 = tmp.path().join("out3");
    overlay(&[base.as_path()], &local, &dest3).unwrap();
    assert_eq!(
        fs::read_to_string(dest3.join("data.json")).unwrap(),
        "{\"n\": 1}"
    );
    let dest4 = tmp.path().join("out4");
    let jbase = mk("jbase", &[("data.json", "{\"n\": 0, \"m\": 2}")]);
    overlay(&[jbase.as_path()], &local, &dest4).unwrap();
    let merged: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dest4.join("data.json")).unwrap()).unwrap();
    assert_eq!(merged["n"], 1);
    assert_eq!(merged["m"], 2);
    // A base dir that doesn't exist errors through payload_files.
    assert!(
        overlay(
            &[tmp.path().join("nope").as_path()],
            &quiet,
            &tmp.path().join("x")
        )
        .is_err()
    );
}

#[test]
fn payload_root_descends_wrappers() {
    let tmp = TempDir::new().unwrap();
    // Manifest at root.
    let direct = tmp.path().join("direct");
    fixture(&direct, "d", None);
    assert_eq!(payload_root(&direct).unwrap(), direct);
    // Wrapped under name-version/.
    let wrapped = tmp.path().join("wrapped");
    fixture(&wrapped.join("n-1.0"), "w", None);
    assert_eq!(payload_root(&wrapped).unwrap(), wrapped.join("n-1.0"));
    // Doubly wrapped.
    let deep = tmp.path().join("deep");
    fixture(&deep.join("a/b"), "x", None);
    assert_eq!(payload_root(&deep).unwrap(), deep.join("a/b"));
    // Widened before a manifest → error.
    let flat = tmp.path().join("flat");
    fs::create_dir_all(&flat).unwrap();
    fs::write(flat.join("a"), "x").unwrap();
    fs::write(flat.join("b"), "x").unwrap();
    assert!(payload_root(&flat).is_err());
    // Single non-dir child → error.
    let single = tmp.path().join("single");
    fs::create_dir_all(&single).unwrap();
    fs::write(single.join("f"), "x").unwrap();
    assert!(payload_root(&single).is_err());
    assert!(payload_root(&tmp.path().join("missing")).is_err());
}

// --- store ----------------------------------------------------------------------

#[test]
fn store_install_find_verify_remove() {
    let tmp = TempDir::new().unwrap();
    let store = Store::new(tmp.path().join("store"));
    assert_eq!(store.root(), tmp.path().join("store"));
    assert!(store.entries().unwrap().is_empty());
    assert!(store.find("nope").is_err());
    // Missing names + labels error differently.
    assert!(store.find("a@1").is_err());

    let payload = tmp.path().join("p");
    fixture(&payload, "gate", Some("0.2.0"));
    let entry = store
        .install(&payload, record("gate", Some("0.2.0")))
        .unwrap();
    assert_eq!(entry.label(), "gate@0.2.0");
    assert_eq!(entry.dir.file_name().unwrap(), "gate@0.2.0");
    assert!(entry.dir.join(".peregrin-store.json").is_file());
    assert_eq!(entry.record.source_ref, "./x");
    assert!(entry.size().unwrap() > 0);
    assert_eq!(entry.manifest().unwrap().name, "gate");

    // find by name + label; entries sorted.
    assert_eq!(store.find("gate").unwrap().label(), "gate@0.2.0");
    assert_eq!(store.find("gate@0.2.0").unwrap().record.name, "gate");
    assert_eq!(store.entries().unwrap().len(), 1);

    // verify ok → tamper → mismatch.
    assert!(store.verify("gate").unwrap().ok);
    fs::write(entry.dir.join("main.yaml"), "tampered: true\n").unwrap();
    let report = store.verify("gate").unwrap();
    assert!(!report.ok);
    assert_ne!(report.recorded, report.computed);

    // A second version → bare name is ambiguous; label removes one.
    let payload2 = tmp.path().join("p2");
    fixture(&payload2, "gate", Some("0.3.0"));
    store
        .install(&payload2, record("gate", Some("0.3.0")))
        .unwrap();
    let err = store.find("gate").unwrap_err().to_string();
    assert!(err.contains("several installs"), "{err}");
    let removed = store.remove("gate@0.2.0").unwrap();
    assert_eq!(removed.name, "gate");
    assert_eq!(store.entries().unwrap().len(), 1);
    assert_eq!(store.find("gate").unwrap().label(), "gate@0.3.0");

    // Unversioned label = short resolved sha; reinstall replaces.
    let payload3 = tmp.path().join("p3");
    fixture(&payload3, "plain", None);
    let e = store.install(&payload3, record("plain", None)).unwrap();
    assert_eq!(e.label(), "plain@deadbeef");
    let again = store.install(&payload3, record("plain", None)).unwrap();
    assert_eq!(store.entries().unwrap().len(), 2);
    assert!(again.dir.is_dir());
    // Corrupt record files fail entries().
    fs::create_dir_all(tmp.path().join("store").join("broken@1")).unwrap();
    fs::write(
        tmp.path()
            .join("store")
            .join("broken@1/.peregrin-store.json"),
        "{ nope",
    )
    .unwrap();
    assert!(store.entries().is_err());
}

#[test]
fn copy_tree_skips_git_and_records() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    fixture(&src, "c", None);
    fs::create_dir_all(src.join(".git")).unwrap();
    fs::write(src.join(".git/x"), "x").unwrap();
    fs::write(src.join(".peregrin-store.json"), "{}").unwrap();
    let dest = tmp.path().join("dest");
    copy_tree(&src, &dest).unwrap();
    assert!(dest.join("peregrin-bundle.toml").is_file());
    assert!(!dest.join(".git").exists());
    assert!(!dest.join(".peregrin-store.json").exists());
    assert!(copy_tree(&tmp.path().join("nope"), &dest).is_err());
}

// --- cache -----------------------------------------------------------------------

#[test]
fn fetch_cache_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let cache = FetchCache::new(tmp.path().join("fetch"));
    assert_eq!(cache.root(), tmp.path().join("fetch"));
    assert!(cache.entries().unwrap().is_empty());
    assert!(!cache.has("k"));
    // scratch → commit → has.
    let scratch = cache.scratch().unwrap();
    assert!(
        scratch
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with('.')
    );
    fs::write(scratch.join("f"), "x").unwrap();
    let dir = cache.commit(&scratch, "sha256-abc").unwrap();
    assert_eq!(dir, cache.entry("sha256-abc"));
    assert!(cache.has("sha256-abc"));
    assert!(dir.join("f").is_file());
    // Existing key wins; scratch dropped.
    let scratch2 = cache.scratch().unwrap();
    assert_eq!(cache.commit(&scratch2, "sha256-abc").unwrap(), dir);
    assert!(!scratch2.exists());
    // Invalid keys.
    for bad in ["", ".", "..", "a/b", "a b", "..x/../y"] {
        assert!(cache.commit(&scratch, bad).is_err(), "{bad}");
        assert!(!cache.has(bad));
    }
    cache.discard(&cache.scratch().unwrap());
    // Dotfiles aren't entries.
    fs::create_dir_all(cache.root().join(".stray")).unwrap();
    assert_eq!(cache.entries().unwrap().len(), 1);
    // clean: a fresh entry survives the default age, ZERO evicts.
    assert!(cache.clean(Duration::from_secs(86_400)).unwrap().is_empty());
    assert_eq!(cache.clean_all().unwrap().len(), 1);
    assert!(cache.entries().unwrap().is_empty());
    // discard on a missing dir is a no-op.
    cache.discard(&tmp.path().join("nope"));
}

#[test]
fn error_display_arms() {
    let e = Error::Failed("f".into());
    assert_eq!(e.to_string(), "f");
    let e = Error::io("ctx")(io::Error::other("boom"));
    assert_eq!(e.to_string(), "ctx: boom");
}

// --- coverage: io error arms ----------------------------------------------------

#[test]
fn io_error_arms_across_archive_overlay_store_and_cache() {
    let tmp = TempDir::new().unwrap();

    // untar: a Regular entry whose target is an existing dir →
    // File::create fails.
    let dest = tmp.path().join("tar-io");
    fs::create_dir_all(dest.join("f")).unwrap();
    let archive = raw_tar(&[("f", b'0', b"x", None)]);
    assert!(untar(archive.as_slice(), &dest).is_err());
    // untar: a symlink whose target name is occupied → symlink fails.
    let dest2 = tmp.path().join("tar-link");
    fs::create_dir_all(&dest2).unwrap();
    fs::write(dest2.join("l"), "occupied").unwrap();
    let archive = raw_tar(&[("l", b'2', b"", Some("x"))]);
    assert!(untar(archive.as_slice(), &dest2).is_err());

    // overlay: a lower-priority file where a higher layer wants a dir
    // → create_dir_all fails.
    let early = tmp.path().join("early");
    fs::create_dir_all(early.join("x")).unwrap();
    fs::write(early.join("x/f"), "x").unwrap();
    let late = tmp.path().join("late");
    fs::create_dir_all(&late).unwrap();
    fs::write(late.join("x"), "file not dir").unwrap();
    let local = tmp.path().join("loc");
    fs::create_dir_all(&local).unwrap();
    assert!(
        overlay(
            &[early.as_path(), late.as_path()],
            &local,
            &tmp.path().join("o1")
        )
        .is_err()
    );
    // overlay: a lower-priority dir where the layer has a same-name
    // file → fs::copy onto a dir fails.
    let early2 = tmp.path().join("early2");
    fs::create_dir_all(&early2).unwrap();
    fs::write(early2.join("z"), "file").unwrap();
    let late2 = tmp.path().join("late2");
    fs::create_dir_all(late2.join("z")).unwrap();
    fs::write(late2.join("z/f"), "x").unwrap();
    assert!(
        overlay(
            &[early2.as_path(), late2.as_path()],
            &local,
            &tmp.path().join("o2")
        )
        .is_err()
    );

    // copy_tree: dest dir where src has a same-name file.
    let src = tmp.path().join("src");
    fixture(&src, "ct", None);
    let dest3 = tmp.path().join("ct");
    fs::create_dir_all(dest3.join("main.yaml")).unwrap();
    assert!(copy_tree(&src, &dest3).is_err());
    // copy_tree: dest file where src wants a dir.
    let dest4 = tmp.path().join("ct2");
    fs::create_dir_all(&dest4).unwrap();
    fs::write(dest4.join("sub"), "file").unwrap();
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub/f"), "x").unwrap();
    assert!(copy_tree(&src, &dest4).is_err());

    // Store entries(): a dir without a record file is skipped.
    let store = Store::new(tmp.path().join("store2"));
    fs::create_dir_all(store.root().join("stray@1")).unwrap();
    let payload = tmp.path().join("pay");
    fixture(&payload, "s", None);
    store.install(&payload, record("s", None)).unwrap();
    assert_eq!(store.entries().unwrap().len(), 1);

    // Cache: a pre-existing scratch dir is cleared on scratch().
    let cache = FetchCache::new(tmp.path().join("fetch2"));
    let stale = cache
        .root()
        .join(format!(".scratch-{}", std::process::id()));
    fs::create_dir_all(&stale).unwrap();
    fs::write(stale.join("junk"), "x").unwrap();
    let scratch = cache.scratch().unwrap();
    assert!(scratch.is_dir());
    assert!(!scratch.join("junk").exists());
    // commit: a missing scratch + fresh key → rename errors.
    assert!(
        cache
            .commit(&tmp.path().join("no-scratch"), "k-new")
            .is_err()
    );
}
