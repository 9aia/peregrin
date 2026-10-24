#![allow(clippy::unwrap_used)]
use peregrin_dotenv::DotenvSecrets;
use peregrin_secret::Secrets;

#[test]
fn get_set_roundtrip() {
    let dir = tempfile::TempDir::new().unwrap();
    let s = DotenvSecrets {
        path: dir.path().join("env"),
    };
    assert!(s.get("A").is_none());
    s.set("A", "1").unwrap();
    s.set("B", "2").unwrap();
    s.set("A", "3").unwrap();
    assert_eq!(s.get("A").as_deref(), Some("3"));
    assert_eq!(s.get("B").as_deref(), Some("2"));
    s.set("C", "").unwrap();
    assert!(s.get("C").is_none());
    let missing = DotenvSecrets {
        path: dir.path().join("nope"),
    };
    assert!(missing.get("A").is_none());
    let bad = DotenvSecrets {
        path: dir.path().join("file").join("x"),
    };
    std::fs::write(dir.path().join("file"), "x").unwrap();
    assert!(bad.set("A", "1").is_err());
}
