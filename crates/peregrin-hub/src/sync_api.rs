//! [`SyncEntityApi`] — the hub's [`EntityApi`] backed by `peregrin-sync`:
//! the entity list reads the local projection (merged across every
//! paired node), detail + detail-stream traffic proxies to the owning
//! node live, and ops go through `SyncEngine::submit_op` so they queue
//! in the durable outbox when the node is down.
//!
//! INTERIM: today's nodes still speak the session plane — op kinds
//! (`sepia.*`), creates (`kind="sepia-session"`), and snapshots map
//! onto `/api/sessions*` routes inside [`peregrin_sync::client`]. When
//! the integration-backed `/api/entities` surface lands (PL-004 phase
//! 4), the shim dissolves and nothing here names a session.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use peregrin_control::{Actor, Arbitration, Decision, LockRegistry, NodeId};
use peregrin_sync::client::NodeClient;
use peregrin_sync::{
    IndexedEntity, NodeRef, NodeTarget, NodeWrite, OpKind, OutboxEntry, OutboxStatus, SyncError,
    SyncHandle,
};

use crate::credentials::NodeCredentials;
use peregrin_node_api::api::EntityApi;
use peregrin_node_api::dto::{
    CreateResultDto, EntityDto, EntityIndexDto, EntityOp, NodeInfoDto, NodeStatusDto, PendingOpDto,
    PushSubscriptionDto,
};
use serde_json::Value;

/// Node registry entry — url + optional bearer token.
#[derive(Clone, Default)]
pub struct HubNode {
    pub id: String,
    pub url: String,
    pub token: Option<String>,
}

/// `PEREGRIN_NODES` format: `id=url[@token];id2=url2[@token2]` — the `@`
/// is unambiguous: a URL never carries one in this context.
pub fn parse_nodes(raw: &str) -> Vec<HubNode> {
    raw.split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (id, rest) = part.split_once('=')?;
            let (url, token) = match rest.split_once('@') {
                Some((u, t)) => (u, if t.is_empty() { None } else { Some(t) }),
                None => (rest, None),
            };
            Some(HubNode {
                id: id.trim().to_string(),
                url: url.trim().trim_end_matches('/').to_string(),
                token: token.map(str::to_string),
            })
        })
        .collect()
}

/// Boot-time token resolution — the [`crate::credentials`] doc has the
/// model. An env `@token`/`PEREGRIN_NODE_TOKEN` is explicit config: it
/// wins for this boot and seeds the store when the node has no stored
/// credential yet — the migration path that lets the plaintext leave
/// the env (drop `@token` after one boot; the token keeps working from
/// the store). A stored credential is never overwritten by env —
/// rotation goes through re-pairing (`adopt_credential` writes the
/// minted token through). Tokenless nodes resolve a stored credential
/// into `node.token`.
///
/// Returns `(resolved, seeded)` for the boot log. Store errors warn
/// and degrade — a broken backend must not keep the hub from booting.
pub fn resolve_node_tokens(nodes: &mut [HubNode], creds: &dyn NodeCredentials) -> (usize, usize) {
    let (mut resolved, mut seeded) = (0usize, 0usize);
    for node in nodes {
        match (node.token.as_deref(), creds.get(&node.id)) {
            (None, Ok(Some(token))) => {
                node.token = Some(token);
                resolved += 1;
            }
            (Some(env_token), Ok(None)) => {
                // Seed an empty slot once — once stored, later boots
                // read Some and skip the write entirely.
                match creds.set(&node.id, env_token) {
                    Ok(()) => seeded += 1,
                    Err(e) => {
                        tracing::warn!(node = %node.id, "credential store write failed: {e}");
                    }
                }
            }
            // env token + stored credential: env wins at runtime, the
            // stored credential (a pair-minted token) stays untouched.
            (Some(_), Ok(Some(_))) | (None, Ok(None)) => {}
            // A failed read can't tell empty from populated — never
            // write blind: an env token must not clobber an unseen
            // stored credential.
            (_, Err(e)) => {
                tracing::warn!(node = %node.id, "credential store lookup failed: {e}");
            }
        }
    }
    (resolved, seeded)
}

/// The node an entity belongs to — projection lookup; entity ids are
/// unique per node but may collide across nodes (`kind` narrows).
fn owner<'a>(
    projection: &peregrin_sync::ProjectionStore,
    id: &str,
    kind: Option<&str>,
    nodes: &'a HashMap<String, HubNode>,
) -> Result<(String, &'a HubNode), String> {
    // Routing policy lives in `peregrin-control` — `kind` narrows when
    // ids collide; otherwise the first match wins.
    let rows: Vec<peregrin_control::OwnershipRow> = projection
        .entities(None)
        .map_err(proj_err)?
        .into_iter()
        .map(|row| peregrin_control::OwnershipRow {
            entity_id: row.entity_id,
            node_id: row.node_id,
            kind: row.kind,
        })
        .collect();
    let node_id = peregrin_control::pick_owner(rows.iter(), id, kind)?.0;
    nodes
        .get(&node_id)
        .map(|n| (node_id.clone(), n))
        .ok_or_else(|| format!("node {node_id} not registered"))
}

/// `owner` + the entity row it picked — callers that need the row skip
/// a second lookup whose `None` arm could never fire.
fn owner_row<'a>(
    projection: &peregrin_sync::ProjectionStore,
    id: &str,
    kind: Option<&str>,
    nodes: &'a HashMap<String, HubNode>,
) -> Result<(String, &'a HubNode, IndexedEntity), String> {
    let rows = projection.entities(None).map_err(proj_err)?;
    let mut found: Option<IndexedEntity> = None;
    for row in rows {
        if row.entity_id != id {
            continue;
        }
        if let Some(want) = kind {
            if row.kind == want {
                found = Some(row);
                break;
            }
        } else if found.is_none() {
            found = Some(row);
        }
    }
    let Some(row) = found else {
        return Err(format!("Entity not found: {id}"));
    };
    let node_id = row.node_id.clone();
    nodes
        .get(&node_id)
        .map(|n| (node_id.clone(), n, row))
        .ok_or_else(|| format!("node {node_id} not registered"))
}

/// The mutable node registry — `pair` rewrites a node's token + client
/// after minting a credential, so reads take the lock.
#[derive(Default)]
struct Registry {
    nodes: HashMap<String, HubNode>,
    clients: HashMap<String, Arc<NodeClient>>,
}

/// The actor every hub-brokered write runs as. One identity covers the
/// hub itself — foreign holders register separately (pid-keyed when
/// the node's lock probe names one) so arbitration can tell "our own
/// hold" from "another actor's".
const HUB_ACTOR: &str = "hub";

/// Op namespaces that act on the live entity the lock protects — the
/// registry gates them before they queue or post. `*.meta*` rides the
/// index overlay and never contends.
fn is_gated_op(op: &str) -> bool {
    // `sepia.<verb>` writes contend; `entity.delete` kills the live
    // entity outright — both gate. `sepia.meta*` is overlay-only.
    op == "entity.delete" || op.starts_with("sepia.") && !op.starts_with("sepia.meta")
}

/// Lock-registry key — `(node, entity)`: entity ids are unique per
/// node but collide across nodes, so the node scopes every grant.
fn lock_key(node_id: &str, entity: &str) -> String {
    format!("{node_id}:{entity}")
}

fn hub_actor(node_id: &str) -> Actor {
    Actor {
        id: HUB_ACTOR.to_string(),
        node: NodeId(node_id.to_string()),
    }
}

/// The holder recorded for a `locked` report the hub never granted —
/// pid-keyed from the index doc's `lockHolderPid` probe when it names
/// one. INTERIM: reads `indexDoc.lockHolderPid`, a session-plane
/// field — the entity wire's lock reporting contract arrives with the
/// integration surface.
fn external_actor(row: &IndexedEntity, node_id: &str) -> Actor {
    let doc = row.raw.get("indexDoc").unwrap_or(&row.raw);
    Actor {
        id: doc
            .get("lockHolderPid")
            .and_then(Value::as_f64)
            .map_or_else(|| "external".to_string(), |pid| format!("pid:{pid}")),
        node: NodeId(node_id.to_string()),
    }
}

/// `EntityApi` over the sync projection + per-node clients.
pub struct SyncEntityApi {
    engine: SyncHandle,
    registry: RwLock<Registry>,
    /// Who holds which entity — the hub is the lock authority; the
    /// projection's `locked` reports plus this hub's own op outcomes
    /// populate it.
    locks: RwLock<LockRegistry>,
    /// The takeover/contention policy rendered at write call sites.
    /// `peregrin-control` decides; the node executes.
    arbitration: Arbitration,
    /// Where `pair` persists minted tokens (keyring or file — picked at
    /// boot by [`crate::credentials::open`]).
    creds: Arc<dyn NodeCredentials>,
    /// First configured node — the canonical target for registry
    /// writes (config/push) that have no entity to route by.
    primary: Option<String>,
}

impl SyncEntityApi {
    pub fn new(engine: SyncHandle, nodes: Vec<HubNode>, creds: Arc<dyn NodeCredentials>) -> Self {
        let clients = nodes
            .iter()
            .filter_map(|n| {
                NodeClient::new(&n.url, n.token.clone())
                    .ok()
                    .map(|c| (n.id.clone(), Arc::new(c)))
            })
            .collect();
        Self {
            engine,
            primary: nodes.first().map(|n| n.id.clone()),
            registry: RwLock::new(Registry {
                nodes: nodes.into_iter().map(|n| (n.id.clone(), n)).collect(),
                clients,
            }),
            locks: RwLock::new(LockRegistry::default()),
            arbitration: Arbitration::default(),
            creds,
        }
    }

    /// Override the arbitration policy — tests and future hub config.
    #[must_use]
    pub fn with_arbitration(mut self, arbitration: Arbitration) -> Self {
        self.arbitration = arbitration;
        self
    }

    fn registry(&self) -> std::sync::RwLockReadGuard<'_, Registry> {
        self.registry
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn registry_mut(&self) -> std::sync::RwLockWriteGuard<'_, Registry> {
        self.registry
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn locks_mut(&self) -> std::sync::RwLockWriteGuard<'_, LockRegistry> {
        self.locks
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Fold the projection's lock report for `row` into the registry:
    /// a reported `locked` registers an external holder the hub never
    /// granted (or refreshes a known one), an unlocked report drops
    /// any recorded hold. The book only ever trails the node's probe.
    fn reconcile_locks(locks: &mut LockRegistry, key: &str, node_id: &str, row: &IndexedEntity) {
        if row.locked {
            match locks.holder(key) {
                Some(held) => {
                    let holder = held.holder.id.clone();
                    locks.refresh(key, &holder);
                }
                None => locks.grant(key, external_actor(row, node_id)),
            }
        } else if let Some(holder) = locks.holder(key).map(|h| h.holder.id.clone()) {
            locks.release(key, &holder);
        }
    }

    /// Reconcile the lock book against `row` and ask the arbitration
    /// policy what the hub may do with `entity` — the single decision
    /// point every gated write consults. Returns the registry key
    /// alongside so callers can record the outcome.
    fn decide(
        &self,
        node_id: &str,
        entity: &str,
        row: &IndexedEntity,
        takeover: bool,
    ) -> (String, Decision) {
        let key = lock_key(node_id, entity);
        let mut locks = self.locks_mut();
        Self::reconcile_locks(&mut locks, &key, node_id, row);
        let decision = self
            .arbitration
            .attach(&locks, &key, &hub_actor(node_id), takeover);
        (key, decision)
    }

    /// The arbitration gate for entity writes — only `Allow` lets a
    /// gated op through; a foreign hold rejects with the holder named
    /// rather than queueing an op the node would refuse on replay.
    fn ensure_writable(
        &self,
        node_id: &str,
        entity: &str,
        row: &IndexedEntity,
    ) -> Result<(), String> {
        let (_key, decision) = self.decide(node_id, entity, row, false);
        match decision {
            Decision::Allow => Ok(()),
            Decision::AttachReadOnly { holder }
            | Decision::Preempt {
                stale_holder: holder,
            } => Err(format!(
                "entity {entity} is held by {holder} on {node_id} — attach with takeover to write"
            )),
            Decision::Queue { node } => Err(format!(
                "entity {entity} is held on {} — attach with takeover to write",
                node.0
            )),
        }
    }

    fn projection(&self) -> peregrin_sync::ProjectionStore {
        self.engine.projection()
    }

    fn client(&self, node_id: &str) -> Result<Arc<NodeClient>, String> {
        self.registry()
            .clients
            .get(node_id)
            .cloned()
            .ok_or_else(|| format!("node {node_id} not registered"))
    }

    /// The client for `node` when given, else the primary's.
    fn scoped_client(&self, node: Option<&str>) -> Result<(String, Arc<NodeClient>), String> {
        let id = match node {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => self
                .primary
                .clone()
                .ok_or_else(|| "no nodes registered".to_string())?,
        };
        let client = self.client(&id)?;
        Ok((id, client))
    }

    /// The upstream `(url, token)` for an entity — the proxy uses it
    /// to route entity-scoped requests to the owning node.
    pub fn node_url_for(&self, id: &str, kind: Option<&str>) -> Option<(String, Option<String>)> {
        let registry = self.registry();
        let (_node_id, node) = owner(&self.projection(), id, kind, &registry.nodes).ok()?;
        Some((node.url.clone(), node.token.clone()))
    }

    fn resolve(&self, id: &str, kind: Option<&str>) -> Result<(String, Arc<NodeClient>), String> {
        let node_id = {
            let registry = self.registry();
            owner(&self.projection(), id, kind, &registry.nodes)?.0
        };
        Ok((node_id.clone(), self.client(&node_id)?))
    }

    /// `/api/sessions/{id}[/{suffix}]` — INTERIM: entity-scoped direct
    /// calls map to the session wire until `/api/entities` lands.
    fn entity_path(id: &str, suffix: &str) -> String {
        let mut path = format!(
            "/api/sessions/{}",
            peregrin_sync::client::encode_segment(id)
        );
        if !suffix.is_empty() {
            path.push('/');
            path.push_str(suffix);
        }
        path
    }

    /// An entity op through the engine — posts direct when the node is
    /// up, enqueues (per-entity FIFO) when down. Gated ops consult the
    /// lock book first.
    async fn submit(
        &self,
        id: &str,
        kind: Option<&str>,
        op: &str,
        policy: OpKind,
        payload: Value,
    ) -> Result<(), String> {
        let (node_id, row) = {
            let registry = self.registry();
            let (node_id, _node, row) = owner_row(&self.projection(), id, kind, &registry.nodes)?;
            (node_id, row)
        };
        if is_gated_op(op) {
            self.ensure_writable(&node_id, id, &row)?;
        }
        self.engine
            .submit_op(NodeWrite {
                node_id: node_id.clone(),
                entity_id: id.to_string(),
                entity_kind: (!row.kind.is_empty()).then(|| row.kind.clone()),
                op: op.to_string(),
                policy,
                payload,
                idempotency_key: uuid::Uuid::new_v4().to_string(),
                ttl_seconds: None,
            })
            .await
            .map_err(err_string)?;
        if op == "entity.delete" {
            // The entity is gone (or queued to be) — drop any recorded
            // hold so a re-created id starts clean.
            let key = lock_key(&node_id, id);
            let mut locks = self.locks_mut();
            if let Some(holder) = locks.holder(&key).map(|h| h.holder.id.clone()) {
                locks.release(&key, &holder);
            }
        }
        Ok(())
    }

    /// Persist a minted credential and start using it: store → registry
    /// (token + rebuilt client) → engine retarget (re-add restarts the
    /// node's sync loop under the new credential). Failures degrade to
    /// in-memory use until restart — the token is still returned to
    /// the caller, so nothing minted is lost.
    async fn adopt_credential(&self, node_id: &str, token: &str) {
        // Blocking store I/O (a dbus round-trip on the keyring backend)
        // stays off the executor.
        let creds = Arc::clone(&self.creds);
        let id = node_id.to_string();
        let tok = token.to_string();
        cred_store_warn(
            tokio::task::spawn_blocking(move || creds.set(&id, &tok)).await,
            node_id,
        );
        let target = {
            let mut registry = self.registry_mut();
            let (node_ref, token_opt) = {
                let Some(entry) = registry.nodes.get_mut(node_id) else {
                    return;
                };
                entry.token = Some(token.to_string());
                (
                    NodeRef {
                        id: entry.id.clone(),
                        url: entry.url.clone(),
                        label: entry.id.clone(),
                    },
                    entry.token.clone(),
                )
            };
            match NodeClient::new(&node_ref.url, token_opt) {
                Ok(client) => {
                    registry
                        .clients
                        .insert(node_id.to_string(), Arc::new(client));
                }
                // The rebuild can't fail for a URL that parsed at boot.
                Err(e) => {
                    tracing::warn!(node = %node_id, "rebuild node client failed: {e}");
                }
            }
            NodeTarget::with_token(node_ref, token)
        };
        retarget_warn(self.engine.add_node(target).await, node_id);
    }

    /// `NodeError` → user-facing text: the node's `{error}` payload
    /// when the body is JSON, else the raw status line.
    fn err_message(e: peregrin_sync::client::NodeError) -> String {
        match e {
            peregrin_sync::client::NodeError::Http { status, body } => {
                serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| format!("http {status}"))
            }
            other => other.to_string(),
        }
    }
}

/// `IndexedEntity` → the DTO — `index_doc` comes out of the raw doc's
/// `indexDoc` slot when the wire wrapped it, else the row itself.
fn entity_dto(row: &IndexedEntity) -> EntityDto {
    EntityDto {
        id: row.entity_id.clone(),
        kind: row.kind.clone(),
        node: Some(row.node_id.clone()),
        index_doc: row
            .raw
            .get("indexDoc")
            .cloned()
            .unwrap_or_else(|| row.raw.clone()),
        locked: row.locked,
        live: row.live,
        updated_at: row.updated_at.clone(),
    }
}

/// `OutboxEntry` → the wire row — pending and in-flight entries read
/// as `queued`; dead-letters surface as `failed` with their last error.
fn pending_dto(e: &OutboxEntry) -> PendingOpDto {
    PendingOpDto {
        id: e.id.clone(),
        node_id: e.node_id.clone(),
        entity_id: e.entity_id.clone(),
        kind: e.op.clone(),
        status: if e.status == OutboxStatus::Dead {
            "failed"
        } else {
            "queued"
        }
        .to_string(),
        enqueued_at: e.created_at.clone(),
        attempts: e.attempts as u32,
        last_error: e.last_error.clone(),
    }
}

#[async_trait::async_trait]
impl EntityApi for SyncEntityApi {
    async fn list_entities(&self, kind: Option<&str>) -> Result<Vec<EntityDto>, String> {
        let rows: Vec<IndexedEntity> = self.projection().entities(None).map_err(proj_err)?;
        Ok(rows
            .iter()
            .filter(|row| kind.is_none_or(|k| row.kind == k))
            .map(entity_dto)
            .collect())
    }

    async fn entity(&self, id: &str) -> Result<EntityDto, String> {
        let registry = self.registry();
        let (_node_id, _node, row) = owner_row(&self.projection(), id, None, &registry.nodes)?;
        Ok(entity_dto(&row))
    }

    async fn create_entity(
        &self,
        kind: &str,
        init: &Value,
        node: Option<&str>,
    ) -> Result<CreateResultDto, String> {
        // INTERIM: `sepia-session` creates map onto the session wire —
        // `init` (the indexDoc) carries the create params the node
        // understands. Other kinds have no registered integration yet.
        if kind != "sepia-session" && kind != "session" {
            return Err(format!("entity kind {kind:?} not registered"));
        }
        let (_node_id, client) = self.scoped_client(node)?;
        let body = init.clone();
        let res =
            tokio::task::spawn_blocking(move || client.send_json("POST", "/api/sessions", &body))
                .await
                .map_err(join_err)?
                .map_err(Self::err_message)?;
        let id = res
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "create returned no entity id".to_string())?;
        Ok(CreateResultDto {
            id: id.to_string(),
            entity: None,
        })
    }

    async fn submit_op(&self, id: &str, op: &EntityOp) -> Result<(), String> {
        // Turn-shaped ops dead-letter on replay failure; meta-shaped
        // ones retry (last-writer-wins is safe for overlay state).
        let policy = if op.kind == "sepia.meta" || op.kind == "sepia.meta.patch" {
            OpKind::Metadata
        } else {
            OpKind::Turn
        };
        self.submit(id, None, &op.kind, policy, op.payload.clone())
            .await
    }

    async fn delete_entity(&self, id: &str) -> Result<(), String> {
        self.submit(
            id,
            None,
            "entity.delete",
            OpKind::Turn,
            serde_json::json!({}),
        )
        .await
    }

    async fn snapshot_export(&self, id: &str) -> Result<Value, String> {
        let (_node_id, client) = self.resolve(id, None)?;
        let path = Self::entity_path(id, "export");
        tokio::task::spawn_blocking(move || client.get_json(&path))
            .await
            .map_err(join_err)?
            .map_err(Self::err_message)
    }

    async fn snapshot_import(&self, body: &Value, node: Option<&str>) -> Result<EntityDto, String> {
        let (node_id, client) = self.scoped_client(node)?;
        let body = body.clone();
        let res = tokio::task::spawn_blocking(move || {
            client.send_json("POST", "/api/sessions/import", &body)
        })
        .await
        .map_err(join_err)?
        .map_err(Self::err_message)?;
        let id = res
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "import returned no entity id".to_string())?;
        Ok(EntityDto {
            id: id.to_string(),
            kind: res
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("sepia-session")
                .to_string(),
            node: Some(node_id),
            index_doc: res,
            ..EntityDto::default()
        })
    }

    async fn entity_index(&self) -> Result<Vec<EntityIndexDto>, String> {
        let projection = self.projection();
        tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            for node in projection.nodes().map_err(proj_err)? {
                let entities = projection
                    .entities(Some(&node.id))
                    .map_err(proj_err)?
                    .iter()
                    .map(entity_dto)
                    .collect();
                out.push(EntityIndexDto {
                    node: node.id,
                    entities,
                });
            }
            Ok(out)
        })
        .await
        .map_err(join_err)?
    }

    async fn pending_ops(&self) -> Result<Vec<PendingOpDto>, String> {
        // Pending entries are addressable per node; dead-letters are a
        // single list. Entries queued for a since-removed node have no
        // reader — the outbox has no scan-all for `pending`.
        let outbox = self.engine.outbox();
        let mut node_ids: Vec<String> = self.registry().nodes.keys().cloned().collect();
        node_ids.sort_unstable();
        tokio::task::spawn_blocking(move || -> Result<Vec<PendingOpDto>, String> {
            let mut out = Vec::new();
            for id in &node_ids {
                for entry in &outbox.pending_for_node(id).map_err(err_string)? {
                    out.push(pending_dto(entry));
                }
            }
            for entry in &outbox.dead_letters().map_err(err_string)? {
                out.push(pending_dto(entry));
            }
            Ok(out)
        })
        .await
        .map_err(join_err)?
    }

    async fn node_info(&self) -> Result<NodeInfoDto, String> {
        let (_, client) = self.scoped_client(None)?;
        tokio::task::spawn_blocking(move || client.get_json("/api/node"))
            .await
            .map_err(join_err)?
            .map_err(err_string)
            .and_then(|v| serde_json::from_value(v).map_err(decode_err))
    }

    async fn rename_node(&self, name: &str, node: Option<&str>) -> Result<NodeInfoDto, String> {
        // Direct post — a rename on a down node can't queue meaningfully
        // (the identity it edits is now, not later), same as pair.
        let (_, client) = self.scoped_client(node)?;
        let body = serde_json::json!({ "name": name });
        tokio::task::spawn_blocking(move || client.send_json("PATCH", "/api/node", &body))
            .await
            .map_err(join_err)?
            .map_err(err_string)
            .and_then(|v| serde_json::from_value(v).map_err(decode_err))
    }

    async fn node_status(&self) -> Result<Vec<NodeStatusDto>, String> {
        let projection = self.projection();
        let registry = self.registry().nodes.clone();
        tokio::task::spawn_blocking(move || {
            let mut rows: Vec<NodeStatusDto> = projection
                .nodes()
                .map_err(proj_err)?
                .into_iter()
                .map(|n| NodeStatusDto {
                    id: n.id,
                    url: n.url,
                    label: n.label,
                    status: n.status.as_str().to_string(),
                    last_seen_at: n.last_seen_at,
                })
                .collect();
            // Registered-but-never-synced nodes still get a row.
            for (id, node) in &registry {
                if !rows.iter().any(|r| &r.id == id) {
                    rows.push(NodeStatusDto {
                        id: id.clone(),
                        url: node.url.clone(),
                        label: id.clone(),
                        status: "unknown".into(),
                        last_seen_at: None,
                    });
                }
            }
            rows.sort_by(|a, b| a.id.cmp(&b.id));
            Ok(rows)
        })
        .await
        .map_err(join_err)?
    }

    async fn pair(&self, code: &str, node: Option<&str>) -> Result<String, String> {
        // Pairing redeems a code on one specific node — `node` picks
        // it, else the primary. Goes direct: the code is single-use
        // and queueing it in the outbox makes no sense.
        let (node_id, client) = self.scoped_client(node)?;
        let body = serde_json::json!({ "code": code });
        let res = tokio::task::spawn_blocking(move || client.send_json("POST", "/api/pair", &body))
            .await
            .map_err(join_err)?
            .map_err(|e| match e {
                // Surface the node's `{error}` payload, not the raw body.
                peregrin_sync::client::NodeError::Http { body, .. } => {
                    serde_json::from_str::<Value>(&body)
                        .ok()
                        .and_then(|v| v.get("error").and_then(Value::as_str).map(str::to_string))
                        .unwrap_or_else(|| format!("pairing failed: {body}"))
                }
                e => e.to_string(),
            })?;
        let token = res
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| "node returned no token".to_string())?
            .to_string();
        self.adopt_credential(&node_id, &token).await;
        Ok(token)
    }

    async fn get_config(&self) -> Result<BTreeMap<String, Value>, String> {
        let (_, client) = self.scoped_client(None)?;
        tokio::task::spawn_blocking(move || {
            let body = client.get_json("/api/config").map_err(err_string)?;
            let map = body
                .get("config")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            Ok(map.into_iter().collect())
        })
        .await
        .map_err(join_err)?
    }

    async fn set_config(&self, key: &str, value: &Value) -> Result<(), String> {
        let (_, client) = self.scoped_client(None)?;
        let path = format!("/api/config/{}", peregrin_sync::client::encode_segment(key));
        let body = serde_json::json!({ "value": value });
        tokio::task::spawn_blocking(move || client.send_json("PATCH", &path, &body))
            .await
            .map_err(join_err)?
            .map(|_| ())
            .map_err(err_string)
    }

    async fn fs_dirs(&self, path: &str, node: Option<String>) -> Result<Vec<String>, String> {
        // Read-only and node-local — goes straight to the named node
        // (or the primary) rather than the projection or outbox.
        let (_, client) = self.scoped_client(node.as_deref())?;
        let api_path = format!(
            "/api/fs?path={}",
            peregrin_sync::client::encode_segment(path)
        );
        tokio::task::spawn_blocking(move || client.get_json(&api_path))
            .await
            .map_err(join_err)?
            .map_err(err_string)
            .map(|v| {
                v.get("dirs")
                    .and_then(Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(|r| r.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            })
    }

    async fn push_vapid(&self) -> Result<String, String> {
        let (_, client) = self.scoped_client(None)?;
        tokio::task::spawn_blocking(move || client.get_json("/api/push/vapid"))
            .await
            .map_err(join_err)?
            .map_err(err_string)
            .and_then(|v| {
                v.get("publicKey")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| "vapid: missing publicKey".to_string())
            })
    }

    async fn push_subscribe(&self, subscription: &PushSubscriptionDto) -> Result<(), String> {
        let (_, client) = self.scoped_client(None)?;
        let body = serde_json::to_value(subscription).map_err(err_string)?;
        tokio::task::spawn_blocking(move || client.send_json("POST", "/api/push/subscribe", &body))
            .await
            .map_err(join_err)?
            .map(|_| ())
            .map_err(err_string)
    }

    async fn push_unsubscribe(&self, endpoint: &str) -> Result<(), String> {
        let (_, client) = self.scoped_client(None)?;
        let body = serde_json::json!({ "endpoint": endpoint });
        tokio::task::spawn_blocking(move || {
            client.send_json("DELETE", "/api/push/subscribe", &body)
        })
        .await
        .map_err(join_err)?
        .map(|_| ())
        .map_err(err_string)
    }
}

/// `e.to_string()` as a fn item — one body shared by every `map_err`
/// tail (the closure per site would need an error per site to score).
fn err_string(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// `format!("decode: {e}")` — see [`err_string`].
fn decode_err(e: impl std::fmt::Display) -> String {
    format!("decode: {e}")
}

/// `format!("projection: {e}")` — see [`err_string`].
fn proj_err(e: impl std::fmt::Display) -> String {
    format!("projection: {e}")
}

/// The `JoinError` arm of `spawn_blocking(…).await` — a panic inside
/// a blocking task is unreachable in practice; a named body tests can
/// call instead of racing a panic.
#[allow(clippy::needless_pass_by_value)]
fn join_err(e: tokio::task::JoinError) -> String {
    e.to_string()
}

/// `engine.add_node` outcome → warn log — split out like
/// `cred_store_warn` so the Shutdown arm is directly testable (an
/// `add_node` failure needs a dead engine, awkward to stage inside a
/// live api).
fn retarget_warn(r: Result<(), SyncError>, node_id: &str) {
    if let Err(e) = r {
        tracing::warn!(node = %node_id, "engine retarget failed: {e}");
    }
}

/// The `adopt_credential` store-write outcome → warn log — split out so
/// the JoinError arm (a panic inside the blocking task) is testable.
fn cred_store_warn(r: Result<Result<(), String>, tokio::task::JoinError>, node_id: &str) {
    let msg = match r {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some(e),
        Err(e) => Some(e.to_string()),
    };
    if let Some(e) = msg {
        tracing::warn!(node = %node_id, "credential store write failed: {e}");
    }
}

#[cfg(test)]
mod cov_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn shared_error_fns() {
        let e = std::io::Error::other("x");
        assert_eq!(err_string(&e), "x");
        assert_eq!(decode_err(&e), "decode: x");
        assert_eq!(proj_err(&e), "projection: x");
        // A real JoinError via abort.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let h = tokio::spawn(async { std::future::pending::<()>().await });
            h.abort();
            let err = h.await.unwrap_err();
            assert!(join_err(err).contains("cancelled"));
        });
    }

    #[tokio::test]
    async fn cred_store_warn_arms() {
        cred_store_warn(Ok(Ok(())), "n1");
        cred_store_warn(Ok(Err("x".into())), "n1");
        let h = tokio::spawn(async { std::future::pending::<Result<(), String>>().await });
        h.abort();
        cred_store_warn(h.await, "n1");
    }

    #[tokio::test]
    async fn adopt_credential_arms() {
        // `node_id` missing from the registry → the early return.
        let (api, _p, _o) = api_for(&[("n1", "http://127.0.0.1:1")]);
        api.adopt_credential("ghost", "tok").await;
        // Re-adopting on an already-registered node → `add_node`
        // retargets cleanly (no warn).
        let (api, _p, _o) = api_for(&[("n1", "http://127.0.0.1:1")]);
        api.adopt_credential("n1", "tok").await;
        // A URL `NodeClient::new` rejects → the rebuild warn arm —
        // the node entered the registry with no client.
        let (api, _p, _o) = api_for(&[("n1", "not a url")]);
        api.adopt_credential("n1", "tok").await;
        // A dead engine → `add_node` Shutdown → the warn arm.
        retarget_warn(Err(SyncError::Shutdown), "n1");
    }

    #[test]
    fn gated_ops_cover_the_live_writes() {
        assert!(is_gated_op("sepia.prompt"));
        assert!(is_gated_op("sepia.permission"));
        assert!(is_gated_op("entity.delete"));
        assert!(!is_gated_op("sepia.meta"));
        assert!(!is_gated_op("sepia.meta.patch"));
    }

    fn api_for(
        nodes: &[(&str, &str)],
    ) -> (
        SyncEntityApi,
        peregrin_sync::ProjectionStore,
        peregrin_outbox::Outbox,
    ) {
        let projection = peregrin_sync::ProjectionStore::in_memory().unwrap();
        let outbox = peregrin_outbox::Outbox::in_memory().unwrap();
        let nodes: Vec<HubNode> = nodes
            .iter()
            .map(|(id, url)| HubNode {
                id: (*id).to_string(),
                url: (*url).to_string(),
                token: None,
            })
            .collect();
        let engine = peregrin_sync::SyncEngine::spawn(
            projection.clone(),
            outbox.clone(),
            nodes.iter().map(|n| {
                NodeTarget::new(peregrin_sync::NodeRef {
                    id: n.id.clone(),
                    url: n.url.clone(),
                    label: n.id.clone(),
                })
            }),
            peregrin_sync::engine::SyncOptions::default(),
        );
        let dir = tempfile::tempdir().unwrap();
        let creds = Arc::new(crate::credentials::FileCredentials::new(
            dir.keep().join("credentials.json"),
        ));
        (SyncEntityApi::new(engine, nodes, creds), projection, outbox)
    }
}
