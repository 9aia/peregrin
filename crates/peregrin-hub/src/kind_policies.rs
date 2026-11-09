//! The hub's entity-kind registrations — one [`KindPolicy`] per
//! integration the gateway fronts. These are *data + small functions
//! over `serde_json::Value`*: they name kind tags and wire field
//! names, never session/campaign types, so the generic machinery in
//! `peregrin-sync` (engine, client, projection, `SyncEntityApi`)
//! stays kind-agnostic.
//!
//! Registration is explicit: [`default_kind_policies`] builds the
//! registry `hub_state_from` hands to the sync engine, the feed hook,
//! and [`SyncEntityApi`]. A host fronts different integrations by
//! registering different policies — nothing here is hardcoded into
//! the gateway paths.

use std::sync::Arc;

use peregrin_sync::kind::{
    DiffTombstone, IndexColumns, KindPolicies, KindPolicy, Notice, NoticeKind,
};
use peregrin_sync::{IndexedEntity, OpKind};
use serde_json::Value;

/// `kind="sepia-session"` — the session integration's sync-side
/// policy. Every rule mirrors the wire contract the session plane
/// has always spoken:
///
/// - `session`/`meta` feed diffs project the entity index; a `meta`
///   tombstone refetches (the overlay died, the entity lives), a
///   `session` tombstone deletes the row.
/// - `sepia.*` ops gate on the entity lock except the `sepia.meta*`
///   overlay; `sepia.meta`/`sepia.meta.patch` replay as metadata,
///   everything else as turns.
/// - Entity-scoped node calls carry the interim `?agent=` disambiguator
///   (the caller's kind hint forwarded verbatim); the live stream is
///   `/api/sessions/{id}/stream`.
/// - `indexDoc.lockHolderPid` names a foreign lock holder; `busy`
///   reads as `live` in the projection columns.
/// - `runFinished`/`permissionRequested` patch flags become push
///   notices titled from `indexDoc.title`.
pub struct SepiaSessionPolicy;

impl KindPolicy for SepiaSessionPolicy {
    fn entity_kinds(&self) -> &'static [&'static str] {
        &["sepia-session"]
    }

    fn diff_events(&self) -> &'static [&'static str] {
        &["session", "meta"]
    }

    fn tombstone(&self, event: &str) -> DiffTombstone {
        match event {
            // Overlay removed — the entity survives; refetch reconciles.
            "meta" => DiffTombstone::Refetch,
            _ => DiffTombstone::DeleteEntity,
        }
    }

    fn owns_op(&self, op: &str) -> bool {
        op.starts_with("sepia.")
    }

    fn op_gated(&self, op: &str) -> bool {
        op.starts_with("sepia.") && !op.starts_with("sepia.meta")
    }

    fn op_kind(&self, op: &str) -> OpKind {
        // Exact-match pair — `sepia.metadata`-style prefixes stay Turn,
        // mirroring the long-standing classification.
        if matches!(op, "sepia.meta" | "sepia.meta.patch") {
            OpKind::Metadata
        } else {
            OpKind::Turn
        }
    }

    fn call_params(&self, scope: Option<&str>) -> Vec<(String, String)> {
        // The interim disambiguator — the entity kind hint as `agent`.
        scope
            .map(|k| vec![("agent".to_string(), k.to_string())])
            .unwrap_or_default()
    }

    fn stream_path(&self, id: &str) -> Option<String> {
        Some(format!("/api/sessions/{id}/stream"))
    }

    fn external_actor_id(&self, row: &IndexedEntity) -> Option<String> {
        let doc = row.raw.get("indexDoc").unwrap_or(&row.raw);
        doc.get("lockHolderPid")
            .and_then(Value::as_f64)
            .map(|pid| format!("pid:{pid}"))
    }

    fn columns(&self, raw: &Value) -> Option<IndexColumns> {
        // `busy` is the session wire's live flag — the generic rules
        // read `live` only.
        let doc = raw.get("indexDoc").filter(|d| d.is_object()).unwrap_or(raw);
        let flag = |k: &str| doc.get(k).and_then(Value::as_bool).unwrap_or(false);
        Some(IndexColumns {
            live: Some(flag("live") || flag("busy")),
            ..IndexColumns::default()
        })
    }

    fn create_kind(&self, requested: &str) -> Option<&'static str> {
        match requested {
            "sepia-session" | "session" => Some("sepia-session"),
            _ => None,
        }
    }

    fn notices(
        &self,
        event: &str,
        entity_id: &str,
        row: Option<&IndexedEntity>,
        patch: &Value,
    ) -> Vec<Notice> {
        // Only `session` diffs notify — `meta` overlay patches never do.
        if event != "session" {
            return Vec::new();
        }
        let title = row.map_or_else(
            || entity_id.to_string(),
            |r| {
                let doc = r.raw.get("indexDoc").unwrap_or(&r.raw);
                doc.get("title")
                    .and_then(Value::as_str)
                    .unwrap_or(entity_id)
                    .to_string()
            },
        );
        let url = format!("/?session={entity_id}");
        let mut out = Vec::new();
        if patch.get("runFinished") == Some(&Value::Bool(true)) {
            out.push(Notice {
                kind: NoticeKind::Done,
                title: "Session finished".into(),
                body: format!("{title} finished its run."),
                url: url.clone(),
            });
        }
        if patch.get("permissionRequested") == Some(&Value::Bool(true)) {
            out.push(Notice {
                kind: NoticeKind::Permission,
                title: "Approval needed".into(),
                body: format!("{title} is waiting for you."),
                url,
            });
        }
        out
    }
}

/// `kind="campaign"` — the orchestration entity's sync-side policy.
/// Campaigns ride the generic contract end-to-end: no feed diff
/// events (the node's `/api/events` feed is session-shaped today),
/// no lock-gated ops, no push notices, and the projection's generic
/// column rules apply. Registered so creates/ops resolve through the
/// same mechanism as every other kind — behavior identical to the
/// unregistered fallback it replaces.
pub struct CampaignKindPolicy;

impl KindPolicy for CampaignKindPolicy {
    fn entity_kinds(&self) -> &'static [&'static str] {
        &["campaign"]
    }
}

/// The registry the hub boots with — sepia-session first so it stays
/// the default kind for kind-less creates/imports.
pub fn default_kind_policies() -> KindPolicies {
    let mut policies = KindPolicies::new();
    policies.register(Arc::new(SepiaSessionPolicy));
    policies.register(Arc::new(CampaignKindPolicy));
    policies
}
