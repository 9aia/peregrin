#![allow(clippy::unwrap_used, clippy::pedantic)]

//! The hub's node credential store — file backend round-trips, on-disk
//! permissions, and backend selection. The real Secret Service can't be
//! forced in CI, so keyring selection is asserted against the live
//! probe result rather than a fixed backend.

use std::collections::HashMap;

use peregrin_hub::credentials::{
    self, FileCredentials, KeyringCredentials, NodeCredentials, STORE_ENV,
};

fn lookup(vars: &HashMap<String, String>) -> impl Fn(&str) -> Option<String> + '_ {
    |k| vars.get(k).cloned()
}

#[test]
fn file_store_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    let store = FileCredentials::new(path.clone());
    assert_eq!(store.get("n1").unwrap(), None);

    store.set("n1", "tok-1").unwrap();
    store.set("n2", "tok-2").unwrap();

    // A fresh instance reads the same file — persistence, not memory.
    let reopened = FileCredentials::new(path);
    assert_eq!(reopened.get("n1").unwrap().as_deref(), Some("tok-1"));
    assert_eq!(reopened.get("n2").unwrap().as_deref(), Some("tok-2"));

    reopened.set("n1", "tok-3").unwrap();
    assert_eq!(store.get("n1").unwrap().as_deref(), Some("tok-3"));

    // Delete is idempotent.
    store.delete("n1").unwrap();
    store.delete("n1").unwrap();
    assert_eq!(store.get("n1").unwrap(), None);
}

#[cfg(unix)]
#[test]
fn file_store_is_written_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    let store = FileCredentials::new(path.clone());
    store.set("n1", "tok").unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "credentials.json must not be world-readable");
}

#[test]
fn file_store_surfaces_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("credentials.json");
    std::fs::write(&path, "not json").unwrap();
    let store = FileCredentials::new(path.clone());
    assert!(store.get("n1").is_err());

    // Non-object JSON fails too; a missing file reads as empty.
    std::fs::write(&path, "[1,2]").unwrap();
    assert!(store.get("n1").is_err());
    std::fs::remove_file(&path).unwrap();
    assert_eq!(store.get("n1").unwrap(), None);
}

#[test]
fn selection_honors_forced_backends() {
    let dir = tempfile::tempdir().unwrap();

    let vars = HashMap::from([(STORE_ENV.to_string(), "file".to_string())]);
    let store = credentials::from_lookup(dir.path(), lookup(&vars));
    assert_eq!(store.backend(), "file");

    // Forced keyring returns the keyring even when the probe fails —
    // explicit config; per-call errors surface honestly at use.
    let vars = HashMap::from([(STORE_ENV.to_string(), "keyring".to_string())]);
    let store = credentials::from_lookup(dir.path(), lookup(&vars));
    assert_eq!(store.backend(), "os-keyring");
}

#[test]
fn auto_picks_keyring_when_reachable_file_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let want = if credentials::keyring_probe().is_ok() {
        "os-keyring"
    } else {
        "file"
    };
    for (name, vars) in [
        ("unset", HashMap::new()),
        (
            "auto",
            HashMap::from([(STORE_ENV.to_string(), "auto".into())]),
        ),
        (
            "empty",
            HashMap::from([(STORE_ENV.to_string(), " ".into())]),
        ),
        (
            "bogus",
            HashMap::from([(STORE_ENV.to_string(), "bogus".into())]),
        ),
    ] {
        let store = credentials::from_lookup(dir.path(), lookup(&vars));
        assert_eq!(store.backend(), want, "{name} should behave like auto");
    }
}

#[test]
fn keyring_store_shape() {
    // No real Secret Service is guaranteed — only that the impl routes
    // through the trait and reports its backend. A working probe means
    // a set/get round-trip is also safe to assert.
    let store = KeyringCredentials;
    assert_eq!(store.backend(), "os-keyring");
    if credentials::keyring_probe().is_err() {
        return;
    }
    store.set("peregrin-hub-probe", "tok").unwrap();
    assert_eq!(
        store.get("peregrin-hub-probe").unwrap().as_deref(),
        Some("tok")
    );
    store.delete("peregrin-hub-probe").unwrap();
    assert_eq!(store.get("peregrin-hub-probe").unwrap(), None);
}

/// Everything keyring, one process: the mock backend answers the
/// happy paths; a failing builder drives the `Err(e)` arms. One test
/// because the default builder is process-global.
#[test]
fn keyring_mock_and_failing_backends() {
    use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi};

    #[derive(Debug)]
    struct FailCred;
    impl CredentialApi for FailCred {
        fn set_secret(&self, _: &[u8]) -> keyring::Result<()> {
            Err(keyring::Error::Invalid("boom".into(), "x".into()))
        }
        fn get_secret(&self) -> keyring::Result<Vec<u8>> {
            Err(keyring::Error::Invalid("boom".into(), "x".into()))
        }
        fn delete_credential(&self) -> keyring::Result<()> {
            Err(keyring::Error::Invalid("boom".into(), "x".into()))
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    #[derive(Debug)]
    struct FailBuilder;
    impl CredentialBuilderApi for FailBuilder {
        fn build(
            &self,
            _target: Option<&str>,
            _service: &str,
            _user: &str,
        ) -> keyring::Result<Box<Credential>> {
            Ok(Box::new(FailCred))
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    let store = KeyringCredentials;
    assert_eq!(store.backend(), "os-keyring");

    // Mock backend — get/set/delete happy paths + NoEntry arms.
    // `keyring::mock` credentials don't share state across Entry
    // instances (set on one, get on another → NoEntry), so this test
    // installs a builder backed by a shared map instead.
    type MockMap = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>;
    #[derive(Debug)]
    struct SharedCred {
        map: MockMap,
        key: String,
    }
    impl CredentialApi for SharedCred {
        fn set_secret(&self, s: &[u8]) -> keyring::Result<()> {
            self.map
                .lock()
                .unwrap()
                .insert(self.key.clone(), s.to_vec());
            Ok(())
        }
        fn get_secret(&self) -> keyring::Result<Vec<u8>> {
            self.map
                .lock()
                .unwrap()
                .get(&self.key)
                .cloned()
                .ok_or(keyring::Error::NoEntry)
        }
        fn delete_credential(&self) -> keyring::Result<()> {
            if self.map.lock().unwrap().remove(&self.key).is_some() {
                Ok(())
            } else {
                Err(keyring::Error::NoEntry)
            }
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    #[derive(Debug)]
    struct SharedBuilder(MockMap);
    impl CredentialBuilderApi for SharedBuilder {
        fn build(
            &self,
            _target: Option<&str>,
            _service: &str,
            user: &str,
        ) -> keyring::Result<Box<Credential>> {
            Ok(Box::new(SharedCred {
                map: self.0.clone(),
                key: user.to_string(),
            }))
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    keyring::set_default_credential_builder(Box::new(SharedBuilder(MockMap::default())));
    assert!(credentials::keyring_probe().is_ok());
    assert_eq!(store.get("n-mock").unwrap(), None);
    store.set("n-mock", "tok-9").unwrap();
    assert_eq!(store.get("n-mock").unwrap().as_deref(), Some("tok-9"));
    store.delete("n-mock").unwrap();
    store.delete("n-mock").unwrap();
    let dir = tempfile::tempdir().unwrap();
    // `auto` + live probe → keyring selected.
    let sel = credentials::from_lookup(dir.path(), |_| None);
    assert_eq!(sel.backend(), "os-keyring");

    // Failing backend — every entry errors → `Err(e)` arms.
    keyring::set_default_credential_builder(Box::new(FailBuilder));
    assert!(credentials::keyring_probe().is_err());
    assert!(store.get("n-err").is_err());
    assert!(store.set("n-err", "t").is_err());
    assert!(store.delete("n-err").is_err());
    // `keyring` forced with a dead probe → warn + keyring anyway.
    let mut vars = HashMap::new();
    vars.insert(STORE_ENV.to_string(), "keyring".to_string());
    let sel = credentials::from_lookup(dir.path(), lookup(&vars));
    assert_eq!(sel.backend(), "os-keyring");
    // `auto` + dead probe → file fallback (log line).
    vars.insert(STORE_ENV.to_string(), "auto".to_string());
    let sel = credentials::from_lookup(dir.path(), lookup(&vars));
    assert_eq!(sel.backend(), "file");
    // Unknown value → warn + file fallback.
    vars.insert(STORE_ENV.to_string(), "junk".to_string());
    let sel = credentials::from_lookup(dir.path(), lookup(&vars));
    assert_eq!(sel.backend(), "file");
    // `file` forced.
    vars.insert(STORE_ENV.to_string(), "file".to_string());
    let sel = credentials::from_lookup(dir.path(), lookup(&vars));
    assert_eq!(sel.backend(), "file");
}
