#![allow(clippy::unwrap_used, clippy::pedantic)]

use peregrin_push::{PushPrefs, PushStore, PushSubscription, SubscriptionKeys};
use sepia_meta::MetaStore;
use serde_json::json;

fn store() -> (PushStore, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    (PushStore::open(meta), dir)
}

fn sub(endpoint: &str) -> PushSubscription {
    PushSubscription {
        endpoint: endpoint.into(),
        keys: SubscriptionKeys {
            auth: "auth".into(),
            p256dh: "p256dh".into(),
        },
        prefs: PushPrefs::default(),
    }
}

#[test]
fn vapid_keys_persist_across_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let meta = MetaStore::open(&dir.path().join("meta.json"));
    let a = PushStore::open(meta.clone());
    let b = PushStore::open(meta);
    assert_eq!(a.public_key(), b.public_key());
    assert!(!a.public_key().is_empty());
}

#[test]
fn upsert_replaces_by_endpoint_and_remove_works() {
    let (push, _dir) = store();
    push.upsert(sub("https://push.example/1"));
    push.upsert(sub("https://push.example/2"));
    let mut replaced = sub("https://push.example/1");
    replaced.prefs.done = false;
    push.upsert(replaced);
    assert_eq!(push.list().len(), 2);
    assert!(
        !push
            .list()
            .iter()
            .find(|s| s.endpoint.ends_with("/1"))
            .unwrap()
            .prefs
            .done
    );
    push.remove("https://push.example/1");
    assert_eq!(push.list().len(), 1);
}

#[test]
fn subscriptions_persist_to_meta_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("meta.json");
    let meta = MetaStore::open(&path);
    let push = PushStore::open(meta.clone());
    push.upsert(sub("https://push.example/1"));
    // Reopen — the config blob round-trips.
    let meta2 = MetaStore::open(&path);
    let push2 = PushStore::open(meta2);
    assert_eq!(push2.list().len(), 1);
    // And the raw file carries the key.
    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        raw["config"]["pushSubscriptions"][0]["endpoint"],
        json!("https://push.example/1")
    );
}

#[test]
fn send_is_a_noop_with_no_matching_prefs() {
    let (push, _dir) = store();
    let mut s = sub("https://push.example/none");
    s.prefs.permission = false;
    push.upsert(s);
    // No recipients → nothing sent (and no panic without a live endpoint).
    assert_eq!(push.send(peregrin_push::Kind::Permission, "t", "b", "/"), 0);
}

/// `{}` deserializes to the opt-in defaults — the `default_true`
/// arms of both prefs.
#[test]
fn prefs_default_to_opted_in() {
    let prefs: PushPrefs = serde_json::from_str("{}").unwrap();
    assert!(prefs.done);
    assert!(prefs.permission);
}

/// A web-push-sendable subscription: real P-256 key material so the
/// payload encrypts and the HTTP POST actually goes on the wire.
fn sendable_sub(endpoint: &str) -> PushSubscription {
    use base64::Engine;
    let enc = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let signing = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);
    let point = signing.verifying_key().to_encoded_point(false);
    PushSubscription {
        endpoint: endpoint.into(),
        keys: SubscriptionKeys {
            auth: enc.encode([7u8; 16]),
            p256dh: enc.encode(point.as_bytes()),
        },
        // `PushPrefs::default()` opts out — the serde `default_true`
        // arms only apply on deserialize.
        prefs: PushPrefs {
            done: true,
            permission: true,
        },
    }
}

/// One-shot HTTP stub: accepts one connection, answers `status`,
/// returns the bound port.
fn stub_push_server(status: &str) -> u16 {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let status = status.to_string();
    std::thread::spawn(move || {
        if let Ok((mut conn, _)) = listener.accept() {
            use std::io::{Read, Write};
            let mut buf = [0u8; 8192];
            let _ = conn.read(&mut buf);
            let _ = conn
                .write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\n\r\n").as_bytes());
        }
    });
    port
}

#[test]
fn send_prunes_expired_endpoints_and_counts_failures() {
    let (push, _dir) = store();
    // A 404 upstream → the error body carries "404" → pruned.
    let gone = stub_push_server("404 Not Found");
    push.upsert(sendable_sub(&format!("http://127.0.0.1:{gone}/push")));
    // A dead port → a transport failure that is neither 404 nor 410 —
    // the `Err(_)` arm keeps the subscription.
    push.upsert(sendable_sub("http://127.0.0.1:1/push"));
    let delivered = push.send(peregrin_push::Kind::Done, "t", "b", "/");
    assert_eq!(delivered, 0);
    let left = push.list();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].endpoint, "http://127.0.0.1:1/push");
}

#[test]
fn send_delivers_to_live_endpoint() {
    let (push, _dir) = store();
    // 200 upstream → the `Ok(())` arm counts the delivery.
    let ok = stub_push_server("200 OK");
    push.upsert(sendable_sub(&format!("http://127.0.0.1:{ok}/push")));
    assert_eq!(push.send(peregrin_push::Kind::Done, "t", "b", "/"), 1);
    assert_eq!(push.list().len(), 1);
}

#[test]
fn send_attempts_build_failures_are_kept() {
    let (push, _dir) = store();
    // Garbage key material — `WebPushMessageBuilder::build` fails
    // before any HTTP, and the `Err(_)` arm keeps the row.
    push.upsert(sub("http://127.0.0.1:1/push"));
    assert_eq!(push.send(peregrin_push::Kind::Done, "t", "b", "/"), 0);
    assert_eq!(push.list().len(), 1);
}

#[test]
fn notify_maps_events_to_notifications() {
    let (push, _dir) = store();
    // A dead-endpoint sub so `send` walks its full path for both kinds.
    push.upsert(sendable_sub("http://127.0.0.1:1/push"));
    let events = [
        sepia_proto::SessionEvent::RunFinished {
            thread_id: "s1".into(),
            run_id: "r1".into(),
        },
        sepia_proto::SessionEvent::Custom {
            name: "acp:permission_request".into(),
            value: serde_json::json!({}),
        },
    ];
    // `agent` joins the session key with ':' — urlencoding escapes it.
    peregrin_push::notify_for_events(&push, "s 1", Some("devin"), "T", &events);
}
