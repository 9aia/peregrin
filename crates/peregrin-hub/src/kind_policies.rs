//! The hub's entity-kind registrations — one [`KindPolicy`] per
//! integration the gateway fronts. These are *data + small functions
//! over `serde_json::Value`*: they name kind tags and wire field
//! names, never session/campaign types, so the generic machinery in
//! `hypha-sync` (engine, client, projection, `SyncEntityApi`)
//! stays kind-agnostic.
//!
//! Registration is explicit: [`default_kind_policies`] builds the
//! registry `hub_state_from` hands to the sync engine, the feed hook,
//! and [`SyncEntityApi`]. A host fronts different integrations by
//! registering different policies — nothing here is hardcoded into
//! the gateway paths.

use std::sync::Arc;

use hypha_sync::kind::{DiffTombstone, IndexColumns, KindPolicies, KindPolicy, Notice, NoticeKind};
use hypha_sync::{IndexedEntity, OpKind};
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
/// Campaigns ride the generic contract (no feed diff events — the
/// node's `/api/events` feed is session-shaped today; no push
/// notices; generic column rules), plus `campaign.*` op arbitration:
///
/// - Every `campaign.*` op is a mutation, so all of them contend for
///   the entity lock (`campaign.start`/`resume` launch a runner,
///   `pause`/`cancel`/`signal` mutate a live one).
/// - `campaign.pause`/`campaign.cancel` are idempotent state
///   transitions — a replayed pause re-parks or re-injects a cancel
///   the runner consumes harmlessly, a replayed cancel lands on
///   `not_running` — so they classify [`OpKind::Metadata`] (retry).
///   `campaign.start`/`resume`/`signal` are one-shot: a duplicate
///   start/resume conflicts on `running`, a duplicate signal injects
///   the user event twice — [`OpKind::Turn`] dead-letters them on
///   replay failure rather than retrying into a conflict.
pub struct CampaignKindPolicy;

impl KindPolicy for CampaignKindPolicy {
    fn entity_kinds(&self) -> &'static [&'static str] {
        &["campaign"]
    }

    fn owns_op(&self, op: &str) -> bool {
        op.starts_with("campaign.")
    }

    fn op_gated(&self, op: &str) -> bool {
        op.starts_with("campaign.")
    }

    fn op_kind(&self, op: &str) -> OpKind {
        match op {
            "campaign.pause" | "campaign.cancel" => OpKind::Metadata,
            _ => OpKind::Turn,
        }
    }
}

/// `kind="ci-job"` — bupha's CI-job entity (P-002, the
/// bupha↔hypha/peregrin seam). The plane's own CAS makes every
/// transition single-threaded on the owning node, so `ci.*` ops are
/// deliberately **not** lock-gated: `ci.renew`/`ci.complete` must
/// reach a `running` (locked) job — gating them on the entity lock
/// would deadlock the claim holder against itself.
///
/// Outbox classification: `ci.enqueue`/`ci.cancel` are idempotent
/// (`pending` enqueue is a no-op, cancel on terminal is a no-op) so
/// they retry as [`OpKind::Metadata`]; `ci.claim`/`ci.renew`/
/// `ci.release`/`ci.complete`/`ci.status` are one-shot — a replayed
/// claim 409s on the CAS, a replayed complete 409s on the fence —
/// so they [`OpKind::Turn`] dead-letter rather than retry into a
/// conflict.
pub struct CiJobKindPolicy;

impl KindPolicy for CiJobKindPolicy {
    fn entity_kinds(&self) -> &'static [&'static str] {
        &["ci-job"]
    }

    fn owns_op(&self, op: &str) -> bool {
        op.starts_with("ci.")
    }

    fn op_kind(&self, op: &str) -> OpKind {
        match op {
            "ci.enqueue" | "ci.cancel" => OpKind::Metadata,
            _ => OpKind::Turn,
        }
    }
}

/// The registry the hub boots with — sepia-session first so it stays
/// the default kind for kind-less creates/imports.
pub fn default_kind_policies() -> KindPolicies {
    let mut policies = KindPolicies::new();
    policies.register(Arc::new(SepiaSessionPolicy));
    policies.register(Arc::new(CampaignKindPolicy));
    policies.register(Arc::new(CiJobKindPolicy));
    policies
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn campaign_policy_claims_and_gates_campaign_ops() {
        let policy = CampaignKindPolicy;
        for op in [
            "campaign.start",
            "campaign.pause",
            "campaign.resume",
            "campaign.cancel",
            "campaign.signal",
            "campaign.unknown",
        ] {
            assert!(policy.owns_op(op), "{op} should be claimed");
            assert!(policy.op_gated(op), "{op} should contend for the lock");
        }
        // Foreign namespaces stay unclaimed — sepia ops are the other
        // policy's, substrate ops are ungated by kind policies.
        for op in ["sepia.prompt", "entity.delete", "campaign", "other.op"] {
            assert!(!policy.owns_op(op), "{op} should not be claimed");
        }
    }

    #[test]
    fn campaign_op_kind_classification() {
        let policy = CampaignKindPolicy;
        // Idempotent transitions retry; one-shot mutations dead-letter.
        assert_eq!(policy.op_kind("campaign.pause"), OpKind::Metadata);
        assert_eq!(policy.op_kind("campaign.cancel"), OpKind::Metadata);
        for op in [
            "campaign.start",
            "campaign.resume",
            "campaign.signal",
            "campaign.unknown",
        ] {
            assert_eq!(policy.op_kind(op), OpKind::Turn, "{op}");
        }
    }

    #[test]
    fn ci_job_policy_claims_ci_ops_without_lock_gating() {
        let policy = CiJobKindPolicy;
        assert_eq!(policy.entity_kinds(), &["ci-job"][..]);
        for op in [
            "ci.enqueue",
            "ci.claim",
            "ci.renew",
            "ci.release",
            "ci.complete",
            "ci.cancel",
            "ci.status",
        ] {
            assert!(policy.owns_op(op), "{op} should be claimed");
            // Never gated — the plane's CAS is the mutual exclusion;
            // gating would strand renew/complete behind the lock the
            // claim itself holds.
            assert!(!policy.op_gated(op), "{op} must not contend for the lock");
        }
        for op in [
            "sepia.prompt",
            "campaign.start",
            "entity.delete",
            "ci",
            "other.op",
        ] {
            assert!(!policy.owns_op(op), "{op} should not be claimed");
        }
    }

    #[test]
    fn ci_job_op_kind_classification() {
        let policy = CiJobKindPolicy;
        // Idempotent transitions retry; CAS/fence-guarded mutations
        // dead-letter on replay failure.
        assert_eq!(policy.op_kind("ci.enqueue"), OpKind::Metadata);
        assert_eq!(policy.op_kind("ci.cancel"), OpKind::Metadata);
        for op in [
            "ci.claim",
            "ci.renew",
            "ci.release",
            "ci.complete",
            "ci.status",
            "ci.unknown",
        ] {
            assert_eq!(policy.op_kind(op), OpKind::Turn, "{op}");
        }
    }

    #[test]
    fn registry_routes_campaign_ops_to_campaign_policy() {
        let policies = default_kind_policies();
        let policy = policies.for_op("campaign.start").unwrap();
        assert_eq!(policy.entity_kinds(), &["campaign"][..]);
        assert!(policies.for_op("sepia.prompt").is_some());
        assert!(policies.for_op("entity.delete").is_none());
    }
}
