//! Ephemeral-node lifecycle — `PEREGRIN_EPHEMERAL=1` /
//! `peregrin serve --ephemeral` (PL-006 phase P2: CI runners as
//! elastic hypha nodes).
//!
//! An ephemeral node is an ordinary node with a different shutdown
//! contract: on SIGINT/SIGTERM it exports the snapshot of every
//! entity that is `live` or `locked` before the planes close, so a
//! runner that dies mid-job doesn't take its work with it. Two
//! destinations, both best-effort:
//!
//! - **local** — `<PEREGRIN_SNAPSHOT_DIR>/<kind>/<id>.snapshot.json`
//!   (default `<PEREGRIN_HOME>/snapshots`) plus an `index.json`
//!   manifest. Survives as long as `PEREGRIN_HOME` does (a CI workspace
//!   volume, an artifact upload, `actions/upload-artifact`).
//! - **handoff** — when `PEREGRIN_SNAPSHOT_TARGET` names a peer node
//!   (or a hub, with `PEREGRIN_SNAPSHOT_NODE` picking the import
//!   target), each snapshot is POSTed to `/api/entities/snapshot` so
//!   the entity resumes on a node that outlives the runner. The hub
//!   stores no snapshots itself — its `snapshot_import` forwards to a
//!   registered node, which is why the target is "a node or a hub",
//!   never "the hub's disk".
//!
//! What ephemeral does NOT change: auth (`PEREGRIN_TOKEN` bearer +
//! the hub's `PEREGRIN_NODES`/`HYPHA_NODES` listing — node→hub trust
//! is config, never pairing), the `/api/*` surface, or entity-plane
//! semantics. Locks need no explicit cleanup: the hub's lock book is
//! in-memory and arbitrates stale holds (`hypha-control`
//! `Arbitration::stale_after`, default 30s) — a dead node's holds
//! preempt, and the session plane's own `close_all`/lock TTLs already
//! run on every shutdown.

use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use hypha_http::entity_plane::{EntityContext, EntityPlane, KindRegistry};
use hypha_node_api::api::{EntityApi, HttpEntityApi};
use serde_json::{Value, json};

/// Cap on one entity's snapshot body — session IR and campaign logs
/// are small; this only bounds a pathological plane.
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// Where flushed snapshots go — built once at boot from `Env`.
#[derive(Clone)]
pub struct FlushConfig {
    /// `<kind>/<id>.snapshot.json` + `index.json` land under here.
    pub dir: PathBuf,
    /// This node's id — recorded in the manifest for provenance.
    pub node_id: String,
    /// Optional handoff target — `None` = local files only.
    pub target: Option<SnapshotTarget>,
}

/// The handoff endpoint — any [`EntityApi`] impl (the HTTP client in
/// production, `FakeEntityApi` in tests).
#[derive(Clone)]
pub struct SnapshotTarget {
    pub api: Arc<dyn EntityApi>,
    /// `?node=` selector — set when the target is a hub.
    pub node: Option<String>,
}

impl FlushConfig {
    /// Wire the flush from `Env` — `PEREGRIN_SNAPSHOT_DIR`,
    /// `PEREGRIN_SNAPSHOT_TARGET`/`_TOKEN`/`_NODE`.
    #[must_use]
    pub fn from_env(env: &peregrin_http::Env, node_id: String) -> Self {
        Self {
            dir: env.snapshot_dir.clone(),
            node_id,
            target: env.snapshot_target.clone().map(|url| SnapshotTarget {
                api: Arc::new(HttpEntityApi::new(url, env.snapshot_token.clone())),
                node: env.snapshot_node.clone(),
            }),
        }
    }
}

/// What one flush pass did — logged at shutdown and asserted in tests.
#[derive(Debug, Default)]
pub struct FlushReport {
    /// Entities exported (live or locked).
    pub exported: usize,
    /// Snapshot files written under `dir`.
    pub written: usize,
    /// Snapshots accepted by the handoff target.
    pub handed_off: usize,
    /// Entities skipped (export failed, non-JSON, or vanished).
    pub skipped: usize,
    /// Non-fatal problems — file/handoff errors, one string each.
    pub errors: Vec<String>,
}

/// Export every `live`/`locked` entity on every registered kind
/// plane, write the snapshots under `cfg.dir`, and hand them to
/// `cfg.target` when configured. Runs inside the graceful-shutdown
/// window — after the listener stops accepting, before the session
/// plane closes — so exports see the same state the wire did.
///
/// Never fails the shutdown: every error lands in the report and a
/// `tracing::warn!`.
pub async fn flush(kinds: &KindRegistry, cfg: &FlushConfig) -> FlushReport {
    let mut report = FlushReport::default();
    let ctx = EntityContext::for_kinds(kinds.clone());
    let mut manifest = Vec::new();
    // Handoffs run concurrently at the end — a dead target costs one
    // client timeout total, not one per entity.
    let mut handoffs = Vec::new();

    for plane in kinds.planes() {
        let docs = plane.list(&ctx, None).await;
        for doc in docs {
            let Some(id) = doc.get("id").and_then(Value::as_str) else {
                continue;
            };
            let live = doc.get("live").and_then(Value::as_bool).unwrap_or(false);
            let locked = doc.get("locked").and_then(Value::as_bool).unwrap_or(false);
            if !live && !locked {
                continue;
            }
            report.exported += 1;
            match export_one(plane.as_ref(), &ctx, id).await {
                Some((bytes, snapshot)) => {
                    if let Some(file) =
                        write_snapshot(cfg, plane.kind(), id, &bytes, &mut report).await
                    {
                        manifest.push(json!({
                            "id": id,
                            "kind": plane.kind(),
                            "file": file,
                            "live": live,
                            "locked": locked,
                        }));
                    }
                    if let (Some(target), Some(snapshot)) = (&cfg.target, snapshot) {
                        let body = import_body(plane.kind(), &snapshot, &doc);
                        let api = Arc::clone(&target.api);
                        let node = target.node.clone();
                        handoffs
                            .push(async move { api.snapshot_import(&body, node.as_deref()).await });
                    }
                }
                None => report.skipped += 1,
            }
        }
    }

    write_manifest(cfg, &manifest, &mut report).await;

    if !handoffs.is_empty() {
        let results = futures::future::join_all(handoffs).await;
        for result in results {
            match result {
                Ok(_) => report.handed_off += 1,
                Err(e) => {
                    report.errors.push(format!("snapshot handoff failed: {e}"));
                    tracing::warn!("ephemeral snapshot handoff failed: {e}");
                }
            }
        }
    }

    tracing::info!(
        exported = report.exported,
        written = report.written,
        handed_off = report.handed_off,
        skipped = report.skipped,
        "ephemeral snapshot flush complete"
    );
    report
}

/// `plane.export` → `(raw bytes, parsed JSON)`. The parsed half is
/// `None` for non-JSON snapshots — they still write to disk verbatim
/// but can't ride the JSON handoff.
async fn export_one(
    plane: &dyn EntityPlane,
    ctx: &EntityContext,
    id: &str,
) -> Option<(axum::body::Bytes, Option<Value>)> {
    let res = plane.export(ctx, id, None).await;
    if res.status() != axum::http::StatusCode::OK {
        tracing::warn!(
            entity = %id,
            kind = plane.kind(),
            status = %res.status(),
            "snapshot export refused"
        );
        return None;
    }
    let bytes = match axum::body::to_bytes(res.into_body(), MAX_SNAPSHOT_BYTES).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(entity = %id, kind = plane.kind(), "snapshot body read failed: {e}");
            return None;
        }
    };
    let parsed = serde_json::from_slice(&bytes).ok();
    Some((bytes, parsed))
}

/// `<dir>/<kind>/<id>.snapshot.json` — returns the manifest-relative
/// path on success.
async fn write_snapshot(
    cfg: &FlushConfig,
    kind: &str,
    id: &str,
    bytes: &[u8],
    report: &mut FlushReport,
) -> Option<String> {
    let dir = cfg.dir.join(kind);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        report
            .errors
            .push(format!("snapshot dir {}: {e}", dir.display()));
        return None;
    }
    let name = snapshot_name(kind, id);
    let file = dir.join(format!("{name}.snapshot.json"));
    match tokio::fs::write(&file, bytes).await {
        Ok(()) => {
            report.written += 1;
            Some(format!("{kind}/{name}.snapshot.json"))
        }
        Err(e) => {
            report.errors.push(format!("write {}: {e}", file.display()));
            None
        }
    }
}

/// Entity ids are plane-chosen, so the filename is sanitized; an id
/// that needed sanitizing gets a deterministic hash suffix so two
/// entities never overwrite each other's snapshot.
fn snapshot_name(kind: &str, id: &str) -> String {
    let safe = |s: &str| {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
    };
    let name = safe(id);
    if name.is_empty() {
        return format!("{}-entity", safe(kind));
    }
    if name == id {
        return name;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    // `{:016x}` of a u64 truncated to u32 keeps names short; the full
    // hash is overkill for a filename disambiguator.
    format!("{name}-{:08x}", (h.finish() & 0xffff_ffff) as u32)
}

/// `index.json` — the manifest a recovery tool (or human) reads to
/// find what the flush preserved.
async fn write_manifest(cfg: &FlushConfig, entries: &[Value], report: &mut FlushReport) {
    let manifest = json!({
        "node": cfg.node_id,
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "snapshots": entries,
    });
    let bytes = match serde_json::to_vec_pretty(&manifest) {
        Ok(b) => b,
        Err(e) => {
            report.errors.push(format!("manifest encode: {e}"));
            return;
        }
    };
    if let Err(e) = tokio::fs::create_dir_all(&cfg.dir).await {
        report
            .errors
            .push(format!("snapshot dir {}: {e}", cfg.dir.display()));
        return;
    }
    let file = cfg.dir.join("index.json");
    if let Err(e) = tokio::fs::write(&file, &bytes).await {
        report.errors.push(format!("write {}: {e}", file.display()));
    }
}

/// The body POSTed to `/api/entities/snapshot` — the kind's export
/// verbatim plus the identity fields its import needs that the export
/// omits: `kind` (plane selection), `id` (id preservation), and
/// `agent`/`cwd`/`title`/`model` lifted from the entity doc's
/// `indexDoc` when absent. Kinds whose import needs more must
/// self-describe their snapshot (`campaign-snapshot/v1` does).
fn import_body(kind: &str, snapshot: &Value, doc: &Value) -> Value {
    let mut body = snapshot.clone();
    if !body.is_object() {
        return body;
    }
    let Some(obj) = body.as_object_mut() else {
        return body;
    };
    obj.entry("kind".to_string())
        .or_insert_with(|| Value::String(kind.to_string()));
    if let Some(id) = doc.get("id").and_then(Value::as_str) {
        obj.entry("id".to_string())
            .or_insert_with(|| Value::String(id.to_string()));
    }
    if let Some(index) = doc.get("indexDoc").and_then(Value::as_object) {
        for key in ["agent", "cwd", "title", "model"] {
            if !obj.contains_key(key) {
                if let Some(v) = index.get(key) {
                    obj.insert(key.to_string(), v.clone());
                }
            }
        }
    }
    body
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use serde_json::json;

    /// Minimal plane: one live entity + one idle, export returns a
    /// fixed snapshot.
    struct StubPlane;

    #[async_trait::async_trait]
    impl EntityPlane for StubPlane {
        fn kind(&self) -> &'static str {
            "stub-kind"
        }
        async fn list(&self, _: &EntityContext, _: Option<&str>) -> Vec<Value> {
            vec![
                json!({"id":"live-1","kind":"stub-kind","live":true,"locked":true,"indexDoc":{"agent":"devin","cwd":"/w"}}),
                json!({"id":"idle-1","kind":"stub-kind","live":false,"locked":false}),
            ]
        }
        async fn get(&self, _: &EntityContext, _: &str, _: Option<&str>) -> Response {
            StatusCode::NOT_FOUND.into_response()
        }
        async fn create(&self, _: &EntityContext, _: Value) -> Response {
            StatusCode::CREATED.into_response()
        }
        async fn delete(&self, _: &EntityContext, _: &str, _: Option<&str>) -> Response {
            StatusCode::OK.into_response()
        }
        async fn op(
            &self,
            _: &EntityContext,
            _: &str,
            _: &str,
            _: Value,
            _: Option<&str>,
        ) -> Response {
            StatusCode::OK.into_response()
        }
        async fn export(&self, _: &EntityContext, id: &str, _: Option<&str>) -> Response {
            axum::Json(
                json!({"format":"stub/v1","entity":{"id":id,"kind":"stub-kind"},"payload":{"n":1}}),
            )
            .into_response()
        }
        async fn import(&self, _: &EntityContext, _: Value) -> Response {
            StatusCode::CREATED.into_response()
        }
    }

    fn cfg(dir: &std::path::Path) -> FlushConfig {
        FlushConfig {
            dir: dir.to_path_buf(),
            node_id: "node_test".into(),
            target: None,
        }
    }

    #[tokio::test]
    async fn flush_writes_live_locked_only_plus_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        let mut kinds = KindRegistry::new();
        kinds.register(Arc::new(StubPlane));
        let report = flush(&kinds, &cfg(tmp.path())).await;
        assert_eq!(report.exported, 1);
        assert_eq!(report.written, 1);
        assert_eq!(report.skipped, 0);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let file = tmp.path().join("stub-kind/live-1.snapshot.json");
        let body: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(body["format"], "stub/v1");
        assert!(!tmp.path().join("stub-kind/idle-1.snapshot.json").exists());

        let manifest: Value =
            serde_json::from_slice(&std::fs::read(tmp.path().join("index.json")).unwrap()).unwrap();
        assert_eq!(manifest["node"], "node_test");
        assert_eq!(manifest["snapshots"].as_array().unwrap().len(), 1);
        assert_eq!(
            manifest["snapshots"][0]["file"],
            "stub-kind/live-1.snapshot.json"
        );
    }

    #[tokio::test]
    async fn flush_survives_unwritable_dir() {
        // A file where the dir should be → every write errors, nothing panics.
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let mut kinds = KindRegistry::new();
        kinds.register(Arc::new(StubPlane));
        let report = flush(&kinds, &cfg(&blocker)).await;
        assert_eq!(report.exported, 1);
        assert_eq!(report.written, 0);
        assert_ne!(report.errors.len(), 0, "expected write errors");
    }

    #[test]
    fn import_body_enriches_export() {
        let doc = json!({
            "id": "s1", "kind": "sepia-session",
            "indexDoc": {"agent":"devin","cwd":"/w","title":"T"}
        });
        let snap = json!({"session": {"id":"s1"}});
        let body = import_body("sepia-session", &snap, &doc);
        assert_eq!(body["kind"], "sepia-session");
        assert_eq!(body["id"], "s1");
        assert_eq!(body["agent"], "devin");
        assert_eq!(body["cwd"], "/w");
        assert_eq!(body["title"], "T");
        assert!(body.get("session").is_some());

        // Existing fields win — the export's own `id`/`agent` stay.
        let snap2 = json!({"kind":"other","id":"keep","agent":"cline"});
        let body2 = import_body("sepia-session", &snap2, &doc);
        assert_eq!(body2["id"], "keep");
        assert_eq!(body2["agent"], "cline");
        assert_eq!(body2["kind"], "other");
    }

    #[test]
    fn snapshot_names_sanitize() {
        assert_eq!(snapshot_name("campaign", "run-abc"), "run-abc");
        assert_eq!(snapshot_name("k", ""), "k-entity");
        // Sanitized ids get a stable hash suffix — distinct raw ids
        // never collapse to the same file.
        let a = snapshot_name("k", "a/b");
        let b = snapshot_name("k", "a_b");
        assert_ne!(a, b);
        assert!(a.starts_with("a_b-"));
    }
}
