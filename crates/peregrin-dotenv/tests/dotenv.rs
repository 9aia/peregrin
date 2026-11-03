//! Security-regression tests for `DotenvSecrets`.
#![allow(clippy::unwrap_used)]

use peregrin_dotenv::DotenvSecrets;
use peregrin_secret::Secrets;

#[test]
fn set_rejects_injecting_names_and_values() {
    let dir = tempfile::tempdir().unwrap();
    let env = DotenvSecrets {
        path: dir.path().join("test.env"),
    };
    assert!(env.set("A\nB", "1").is_err());
    assert!(env.set("A=B", "1").is_err());
    assert!(env.set("", "1").is_err());
    assert!(env.set("A", "x\nB=evil").is_err());
    assert!(env.set("A", "x\ry").is_err());
    // Nothing written on rejection.
    assert!(!env.path.exists());
}

#[test]
fn get_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    let env = DotenvSecrets {
        path: dir.path().join("test.env"),
    };
    env.set("REAL", "v").unwrap();
    assert_eq!(env.get(""), None);
    assert_eq!(env.get("A\nREAL"), None);
}

#[test]
fn round_trip_and_replace() {
    let dir = tempfile::tempdir().unwrap();
    let env = DotenvSecrets {
        path: dir.path().join("test.env"),
    };
    env.set("A", "1").unwrap();
    env.set("B", "2").unwrap();
    env.set("A", "3").unwrap();
    assert_eq!(env.get("A").as_deref(), Some("3"));
    assert_eq!(env.get("B").as_deref(), Some("2"));
    // The file holds exactly the live entries — no dup lines.
    let body = std::fs::read_to_string(&env.path).unwrap();
    assert_eq!(body, "B=2\nA=3\n");
}

#[cfg(unix)]
#[test]
fn secrets_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let env = DotenvSecrets {
        path: dir.path().join("test.env"),
    };
    env.set("A", "1").unwrap();
    assert_eq!(
        env.path.metadata().unwrap().permissions().mode() & 0o777,
        0o600
    );
    // A rewrite keeps the mode.
    env.set("A", "2").unwrap();
    assert_eq!(
        env.path.metadata().unwrap().permissions().mode() & 0o777,
        0o600
    );
}
