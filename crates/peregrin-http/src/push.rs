//! Web Push wiring for the session plane — the `PushBackend`
//! implementation that keeps subscriptions inside the meta overlay
//! (where they've always lived) plus the session-event → notification
//! mapping. The push *machinery* (VAPID, subscription store, send)
//! is `hypha_push`; what to notify and when stays Peregrin's.

use std::collections::BTreeMap;

use hypha_push::{Kind, PushStore};
use sepia_meta::MetaStore;
use sepia_proto::SessionEvent;
use serde_json::Value;

/// The node's push persistence — the meta overlay's `config` document
/// (`vapid` + `pushSubscriptions` keys), so subscriptions survive in
/// `meta.json` exactly where they always did. A newtype because both
/// `MetaStore` and `PushBackend` are foreign to this crate (orphan
/// rule) — the wrapper is the local type the impl hangs on.
#[derive(Clone)]
pub struct MetaPushBackend(pub MetaStore);

impl hypha_push::PushBackend for MetaPushBackend {
    fn config(&self) -> BTreeMap<String, Value> {
        MetaStore::config(&self.0)
    }

    fn set_config(&self, key: &str, value: Value) {
        MetaStore::set_config(&self.0, key, value);
    }
}

/// Session-event → notification mapping — run completion and permission
/// requests, matching the TS `notifyForEvents`.
pub fn notify_for_events(
    push: &PushStore,
    session_id: &str,
    agent: Option<&str>,
    session_title: &str,
    events: &[SessionEvent],
) {
    let key = agent.map_or_else(|| session_id.to_string(), |a| format!("{a}:{session_id}"));
    let url = format!("/?session={}", urlencoding(&key));
    if events
        .iter()
        .any(|e| matches!(e, SessionEvent::RunFinished { .. }))
    {
        push.send(
            Kind::Done,
            "Session finished",
            &format!("{session_title} finished its run."),
            &url,
        );
    }
    let wants_permission = events.iter().any(
        |e| matches!(e, SessionEvent::Custom { name, .. } if name == "acp:permission_request"),
    );
    if wants_permission {
        push.send(
            Kind::Permission,
            "Approval needed",
            &format!("{session_title} is waiting for you."),
            &url,
        );
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}
