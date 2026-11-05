//! `peregrin-source-http` — real ureq fetches against a one-shot
//! `TcpListener` stub.
#![allow(
    clippy::unwrap_used,
    unknown_lints,
    clippy::assert_is_empty,
    clippy::default_constructed_unit_structs
)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;

use peregrin_bundle::{pack_tgz, sha256_hex};
use peregrin_source::{Resolved, Source, SourceRef, parse_ref};
use peregrin_source_http::HttpSource;
use tempfile::TempDir;

/// Serve `body` once at `status`, returning the tarball URL.
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
    format!("http://127.0.0.1:{port}/bundle.tgz")
}

fn http_ref(url: &str) -> SourceRef {
    parse_ref(url).unwrap()
}

fn tgz_bytes(tmp: &TempDir) -> Vec<u8> {
    let dir = tmp.path().join("bundle");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("peregrin-bundle.yaml"), "name: h\n").unwrap();
    let out = tmp.path().join("b.tgz");
    pack_tgz(&dir, &out).unwrap();
    fs::read(&out).unwrap()
}

#[test]
fn fetch_downloads_verifies_and_unpacks() {
    let tmp = TempDir::new().unwrap();
    let bytes = tgz_bytes(&tmp);
    let digest = sha256_hex(&bytes);
    let url = serve_once("200 OK", bytes);

    let source = HttpSource::new();
    // Unpinned.
    let resolved = source.resolve(&http_ref(&url)).unwrap();
    let dest = tmp.path().join("out");
    let fetched = source.fetch(&resolved, &dest).unwrap();
    assert_eq!(fetched.resolved, format!("sha256:{digest}"));
    assert_eq!(fetched.content_key, format!("sha256-{digest}"));
    assert!(dest.join("peregrin-bundle.yaml").is_file());

    // Pinned — resolve carries the digest, fetch verifies it.
    let url = serve_once("200 OK", tgz_bytes(&tmp));
    let resolved = source
        .resolve(&http_ref(&format!("{url}#sha256:{digest}")))
        .unwrap();
    assert_eq!(
        resolved,
        Resolved::Http {
            url: url.clone(),
            digest: Some(digest.clone())
        }
    );
    assert_eq!(
        resolved.prefetch_identity().unwrap().0,
        format!("sha256-{digest}")
    );
    let dest = tmp.path().join("out2");
    source.fetch(&resolved, &dest).unwrap();
    assert!(dest.join("peregrin-bundle.yaml").is_file());
}

#[test]
fn fetch_error_arms() {
    let tmp = TempDir::new().unwrap();
    let source = HttpSource::new();
    let bytes = tgz_bytes(&tmp);

    // Digest mismatch.
    let url = serve_once("200 OK", bytes);
    let bad = "0".repeat(64);
    let resolved = source
        .resolve(&http_ref(&format!("{url}#sha256:{bad}")))
        .unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("d1"))
            .unwrap_err()
            .to_string()
            .contains("digest mismatch")
    );

    // HTTP error status.
    let url = serve_once("404 Not Found", Vec::new());
    let resolved = source.resolve(&http_ref(&url)).unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("d2"))
            .unwrap_err()
            .to_string()
            .contains("404")
    );

    // A 200 that isn't a tarball fails at unpack.
    let url = serve_once("200 OK", b"not a tarball".to_vec());
    let resolved = source.resolve(&http_ref(&url)).unwrap();
    assert!(source.fetch(&resolved, &tmp.path().join("d3")).is_err());

    // Dead port → ureq error arm.
    let resolved = source
        .resolve(&http_ref("http://127.0.0.1:1/x.tgz"))
        .unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("d4"))
            .unwrap_err()
            .to_string()
            .contains("GET http://127.0.0.1:1/x.tgz")
    );

    // Wrong variants.
    let fs_ref = SourceRef::Fs(PathBuf::from("x"));
    assert!(
        source
            .resolve(&fs_ref)
            .unwrap_err()
            .to_string()
            .contains("only resolves http")
    );
    let git_resolved = Resolved::Git {
        url: "u".into(),
        subdir: None,
        rev: None,
        sha: None,
    };
    assert!(
        source
            .fetch(&git_resolved, &tmp.path().join("d5"))
            .unwrap_err()
            .to_string()
            .contains("only fetches http")
    );
    // with_agent + Default.
    let _ = HttpSource::with_agent(ureq::Agent::new_with_defaults());
    let _ = HttpSource::default();
}

#[test]
fn truncated_body_errors_on_read() {
    // Content-length larger than the bytes sent → read_to_vec's error
    // arm.
    let tmp = TempDir::new().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 8192];
        let _ = stream.read(&mut buf);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 999999\r\n\r\n")
            .unwrap();
        stream.write_all(b"short").unwrap();
        // Close without sending the rest.
    });
    let source = HttpSource::new();
    let resolved = source
        .resolve(&http_ref(&format!("http://127.0.0.1:{port}/b.tgz")))
        .unwrap();
    assert!(
        source
            .fetch(&resolved, &tmp.path().join("d"))
            .unwrap_err()
            .to_string()
            .contains("read")
    );
}
