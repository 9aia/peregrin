//! Per-kind policy registry — what the sync layer knows about each
//! entity kind beyond the generic `/api/entities*` contract.
//!
//! The substrate moves entity docs, ops, and snapshots opaquely; a
//! [`KindPolicy`] carries the kind-specific knowledge the hub needs
//! to do its job: which SSE diff events project the kind, how
//! tombstones resolve, how entity-scoped calls map onto node wire
//! params/stream paths, which ops contend for the entity lock, how
//! outbox replays classify, how a locked row names its external
//! holder, and how wire docs columnize. Integrations register one
//! policy per kind at composition; unregistered kinds get the generic
//! behavior everywhere.
//!
//! Nothing here names a session, campaign, or orchestration type —
//! policies are data + small functions over `serde_json::Value`.

use std::sync::Arc;

use serde_json::Value;

use crate::{IndexedEntity, OpKind};

/// Column values a wire entity doc projects to. Every field is
/// optional — `None` falls back to the generic extraction (the
/// `{id,kind,indexDoc}` wrap unwrap plus the tolerant field reads).
#[derive(Clone, Debug, Default)]
pub struct IndexColumns {
    /// Entity id — defaults to `id` on the doc (or inside `indexDoc`).
    pub entity_id: Option<String>,
    /// Entity kind tag — defaults to `kind` on the doc.
    pub kind: Option<String>,
    /// RFC 3339 `updatedAt`.
    pub updated_at: Option<String>,
    /// Lock-held flag.
    pub locked: Option<bool>,
    /// Live flag.
    pub live: Option<bool>,
}

/// How a `{deleted: true}` index diff resolves for a kind's event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffTombstone {
    /// Drop the projected entity row outright.
    DeleteEntity,
    /// Refetch the entity doc — the row survives (e.g. an overlay was
    /// removed but the entity lives on).
    Refetch,
}

/// Notification channel a [`Notice`] targets — the host maps it onto
/// its notifier (the hub's Web Push has exactly these two).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    /// Terminal/completion notice.
    Done,
    /// Human input requested.
    Permission,
}

/// A user-facing notification a kind wants delivered for a diff —
/// produced by [`KindPolicy::notices`], rendered by the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    /// Channel/category.
    pub kind: NoticeKind,
    /// Notification title.
    pub title: String,
    /// Notification body.
    pub body: String,
    /// Deep-link the notification opens.
    pub url: String,
}

/// One entity kind's sync-side policy. All methods have inert
/// defaults — a minimal policy is `entity_kinds` plus whatever the
/// kind actually needs.
pub trait KindPolicy: Send + Sync {
    /// Entity kind tags this policy serves (`"sepia-session"`,
    /// `"campaign"`, …). A kind is "registered" for create/gating
    /// purposes iff some policy claims it.
    fn entity_kinds(&self) -> &'static [&'static str];

    /// SSE `event:` names whose `{id, patch}` frames are this kind's
    /// index diffs. Empty = the kind only syncs via full listings.
    fn diff_events(&self) -> &'static [&'static str] {
        &[]
    }

    /// Tombstone resolution for one of [`Self::diff_events`].
    fn tombstone(&self, _event: &str) -> DiffTombstone {
        DiffTombstone::DeleteEntity
    }

    /// Does this policy arbitrate `op` (`<ns>.<verb>`)? First
    /// registered claimant wins — keep namespaces disjoint.
    fn owns_op(&self, _op: &str) -> bool {
        false
    }

    /// Does `op` contend for the entity's live lock (gated writes
    /// consult the hub's lock book before queueing/posting)? Only
    /// consulted for ops [`Self::owns_op`] claims; `entity.delete`
    /// is substrate-gated regardless.
    fn op_gated(&self, _op: &str) -> bool {
        false
    }

    /// Outbox replay classification for an owned op —
    /// [`OpKind::Metadata`] retries, [`OpKind::Turn`] dead-letters.
    /// Only consulted for ops [`Self::owns_op`] claims.
    fn op_kind(&self, _op: &str) -> OpKind {
        OpKind::Turn
    }

    /// Query params appended to entity-scoped node calls
    /// (`GET /api/entities/{id}`, `POST …/ops`, stream opens).
    /// `scope` is the caller's hint — today the entity kind value
    /// itself, forwarded verbatim (the interim disambiguator).
    fn call_params(&self, _scope: Option<&str>) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Live event-stream path for the entity — `id` is already
    /// percent-encoded. `None` = the kind has no entity stream.
    fn stream_path(&self, _id: &str) -> Option<String> {
        None
    }

    /// The holder id recorded for a `locked` projection row the hub
    /// never granted — `None` reads as `"external"`. Only consulted
    /// for rows whose `kind` this policy serves.
    fn external_actor_id(&self, _row: &IndexedEntity) -> Option<String> {
        None
    }

    /// Column extraction override for this kind's wire docs — merged
    /// over the generic rules field-by-field. `None` = generic.
    fn columns(&self, _raw: &Value) -> Option<IndexColumns> {
        None
    }

    /// The canonical entity kind a create request for `requested`
    /// maps to — `None` = this policy doesn't serve the create.
    /// Default: the kind itself when it's one of
    /// [`Self::entity_kinds`]; integrations override to accept aliases
    /// (`"session"` → `"sepia-session"`).
    fn create_kind(&self, requested: &str) -> Option<&'static str> {
        self.entity_kinds()
            .iter()
            .copied()
            .find(|k| *k == requested)
    }

    /// Create-payload validation — runs after [`Self::create_kind`]
    /// accepts the kind. `Err` rejects the create before it posts.
    fn validate_create(&self, _init: &Value) -> Result<(), String> {
        Ok(())
    }

    /// User-facing notices this kind wants delivered for a diff
    /// `event` on `entity_id`. `row` is the projected entity when
    /// indexed (`None` — e.g. a diff ahead of the first listing —
    /// implementations fall back to the id). Default: none.
    fn notices(
        &self,
        _event: &str,
        _entity_id: &str,
        _row: Option<&IndexedEntity>,
        _patch: &Value,
    ) -> Vec<Notice> {
        Vec::new()
    }
}

/// `kind`/`op`/`event` → [`KindPolicy`] registry. Registration order
/// matters only for first-claim wins (`owns_op`, `diff_events`,
/// `entity_kinds` lookups); keep namespaces disjoint.
#[derive(Clone, Default)]
pub struct KindPolicies {
    inner: Arc<Vec<Arc<dyn KindPolicy>>>,
}

impl KindPolicies {
    /// An empty registry — every lookup misses, every kind gets the
    /// generic behavior.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a policy. Re-registering a policy whose
    /// `entity_kinds` overlap replaces the earlier claimant in place
    /// (recomposition is idempotent).
    pub fn register(&mut self, policy: Arc<dyn KindPolicy>) {
        let mut inner = Vec::clone(&self.inner);
        let kinds = policy.entity_kinds();
        inner.retain(|p| !p.entity_kinds().iter().any(|k| kinds.contains(k)));
        inner.push(policy);
        self.inner = Arc::new(inner);
    }

    /// Every registered policy, in registration order.
    pub fn policies(&self) -> &[Arc<dyn KindPolicy>] {
        &self.inner
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// The policy serving entity `kind`, if registered.
    pub fn for_entity_kind(&self, kind: &str) -> Option<Arc<dyn KindPolicy>> {
        self.inner
            .iter()
            .find(|p| p.entity_kinds().contains(&kind))
            .cloned()
    }

    /// The policy arbitrating `op`, if any claims it.
    pub fn for_op(&self, op: &str) -> Option<Arc<dyn KindPolicy>> {
        self.inner.iter().find(|p| p.owns_op(op)).cloned()
    }

    /// The diff-event resolution for a feed `event:` name — the
    /// claiming policy plus its tombstone semantics. `None` = the
    /// event isn't any registered kind's index diff (inert).
    pub fn diff_policy(&self, event: &str) -> Option<(Arc<dyn KindPolicy>, DiffTombstone)> {
        self.inner.iter().find_map(|p| {
            p.diff_events()
                .contains(&event)
                .then(|| (Arc::clone(p), p.tombstone(event)))
        })
    }

    /// The policy claiming diff `event` (one of its
    /// [`KindPolicy::diff_events`]) — the feed-hook path. `None` = the
    /// event isn't an index diff any kind projects.
    pub fn for_diff_event(&self, event: &str) -> Option<Arc<dyn KindPolicy>> {
        self.inner
            .iter()
            .find(|p| p.diff_events().contains(&event))
            .cloned()
    }

    /// The canonical kind a create for `requested` maps to — first
    /// policy whose [`KindPolicy::create_kind`] accepts wins.
    pub fn create_kind(&self, requested: &str) -> Option<&'static str> {
        self.inner.iter().find_map(|p| p.create_kind(requested))
    }

    /// The policy serving a create for `requested`, with the canonical
    /// kind it maps to — `None` = no registered integration accepts
    /// the create.
    pub fn for_create(&self, requested: &str) -> Option<(Arc<dyn KindPolicy>, &'static str)> {
        self.inner.iter().find_map(|p| {
            p.create_kind(requested)
                .map(|canonical| (Arc::clone(p), canonical))
        })
    }

    /// The first registered kind — the default for kind-less creates
    /// and imports (mirrors the node-side default plane).
    pub fn default_entity_kind(&self) -> Option<&'static str> {
        self.inner
            .first()
            .and_then(|p| p.entity_kinds().first().copied())
    }

    /// Column extraction for one wire entity doc: the kind's policy
    /// override merged field-by-field over the generic rules. The
    /// generic rules unwrap the `{indexDoc}` interim wrap and read
    /// `updatedAt`/`locked`/`live` tolerantly (`live` also honors the
    /// legacy `busy` alias) — see [`crate::columns_of`].
    pub fn columns_of(&self, raw: &Value) -> IndexedEntity {
        let mut row = crate::columns_of(raw);
        if let Some(cols) = self.for_entity_kind(&row.kind).and_then(|p| p.columns(raw)) {
            if let Some(v) = cols.entity_id {
                row.entity_id = v;
            }
            if let Some(v) = cols.kind {
                row.kind = v;
            }
            if let Some(v) = cols.updated_at {
                row.updated_at = v;
            }
            if let Some(v) = cols.locked {
                row.locked = v;
            }
            if let Some(v) = cols.live {
                row.live = v;
            }
        }
        row
    }
}
