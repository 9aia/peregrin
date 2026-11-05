//! Test fixtures — a deterministic in-memory [`EntityApi`]. `FakeEntityApi::seeded()`
//! returns a fixed world (three entities across two kinds, a node
//! descriptor, an empty outbox); `.with_*` knobs override individual
//! fixtures and `.with_error(..)` makes every read endpoint fail.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::EntityApi;
use crate::dto::{
    CreateResultDto, EntityDto, EntityIndexDto, EntityOp, NodeInfoDto, NodeStatusDto, PendingOpDto,
    PushSubscriptionDto,
};

/// An `EntityApi` that answers every read from in-memory fixtures.
/// Mutations are no-ops returning plausible successes.
#[derive(Clone)]
pub struct FakeEntityApi {
    entities: Vec<EntityDto>,
    node_info: NodeInfoDto,
    nodes: Vec<NodeStatusDto>,
    pending: Vec<PendingOpDto>,
    error: Option<String>,
}

impl Default for FakeEntityApi {
    fn default() -> Self {
        Self::seeded()
    }
}

impl FakeEntityApi {
    /// The canned world.
    pub fn seeded() -> Self {
        Self {
            entities: vec![
                EntityDto {
                    id: "s1".into(),
                    kind: "sepia-session".into(),
                    node: Some("node".into()),
                    index_doc: json!({"title": "fix parser", "cwd": "/work/a"}),
                    live: true,
                    updated_at: "2026-11-01T10:00:00Z".into(),
                    ..EntityDto::default()
                },
                EntityDto {
                    id: "s2".into(),
                    kind: "sepia-session".into(),
                    node: Some("node".into()),
                    index_doc: json!({"title": "docs pass", "cwd": "/work/b"}),
                    locked: true,
                    updated_at: "2026-11-01T09:00:00Z".into(),
                    ..EntityDto::default()
                },
                EntityDto {
                    id: "c1".into(),
                    kind: "campaign".into(),
                    node: Some("tower".into()),
                    index_doc: json!({"title": "nightly sweep"}),
                    updated_at: "2026-11-01T08:00:00Z".into(),
                    ..EntityDto::default()
                },
            ],
            node_info: NodeInfoDto {
                id: "node".into(),
                name: "local".into(),
            },
            nodes: vec![NodeStatusDto {
                id: "node".into(),
                url: "http://127.0.0.1:18787".into(),
                label: "node".into(),
                status: "up".into(),
                last_seen_at: None,
            }],
            pending: Vec::new(),
            error: None,
        }
    }

    /// Replace the entity list.
    #[must_use]
    pub fn with_entities(mut self, entities: Vec<EntityDto>) -> Self {
        self.entities = entities;
        self
    }

    /// Replace the pending-ops fixture.
    #[must_use]
    pub fn with_pending(mut self, pending: Vec<PendingOpDto>) -> Self {
        self.pending = pending;
        self
    }

    /// Every read fails with `msg`.
    #[must_use]
    pub fn with_error(mut self, msg: impl Into<String>) -> Self {
        self.error = Some(msg.into());
        self
    }

    /// Wrap in the shared trait object shape callers expect.
    pub fn shared(self) -> Arc<dyn EntityApi> {
        Arc::new(self)
    }

    fn fail<T>(&self, v: T) -> Result<T, String> {
        self.error.clone().map_or(Ok(v), Err)
    }
}

#[async_trait::async_trait]
impl EntityApi for FakeEntityApi {
    async fn list_entities(&self, kind: Option<&str>) -> Result<Vec<EntityDto>, String> {
        self.fail(
            self.entities
                .iter()
                .filter(|e| kind.is_none_or(|k| e.kind == k))
                .cloned()
                .collect(),
        )
    }
    async fn entity(&self, id: &str) -> Result<EntityDto, String> {
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        self.entities
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or_else(|| format!("entity {id} not found"))
    }
    async fn create_entity(
        &self,
        kind: &str,
        init: &Value,
        _node: Option<&str>,
    ) -> Result<CreateResultDto, String> {
        self.fail(CreateResultDto {
            id: "e-new".into(),
            entity: Some(EntityDto {
                id: "e-new".into(),
                kind: kind.to_string(),
                index_doc: init.clone(),
                ..EntityDto::default()
            }),
        })
    }
    async fn submit_op(&self, _id: &str, _op: &EntityOp) -> Result<(), String> {
        self.fail(())
    }
    async fn delete_entity(&self, _id: &str) -> Result<(), String> {
        self.fail(())
    }
    async fn snapshot_export(&self, _id: &str) -> Result<Value, String> {
        self.fail(json!({}))
    }
    async fn snapshot_import(
        &self,
        body: &Value,
        _node: Option<&str>,
    ) -> Result<EntityDto, String> {
        self.fail(EntityDto {
            id: "e-imported".into(),
            kind: "sepia-session".into(),
            index_doc: body.clone(),
            ..EntityDto::default()
        })
    }
    async fn entity_index(&self) -> Result<Vec<EntityIndexDto>, String> {
        self.fail(vec![EntityIndexDto {
            node: "node".into(),
            entities: self.entities.clone(),
        }])
    }
    async fn pending_ops(&self) -> Result<Vec<PendingOpDto>, String> {
        self.fail(self.pending.clone())
    }
    async fn node_info(&self) -> Result<NodeInfoDto, String> {
        self.fail(self.node_info.clone())
    }
    async fn rename_node(&self, name: &str, _node: Option<&str>) -> Result<NodeInfoDto, String> {
        self.fail(NodeInfoDto {
            id: self.node_info.id.clone(),
            name: name.to_string(),
        })
    }
    async fn node_status(&self) -> Result<Vec<NodeStatusDto>, String> {
        self.fail(self.nodes.clone())
    }
    async fn pair(&self, _code: &str, _node: Option<&str>) -> Result<String, String> {
        self.fail("token".to_string())
    }
    async fn get_config(&self) -> Result<BTreeMap<String, Value>, String> {
        self.fail(BTreeMap::new())
    }
    async fn set_config(&self, _key: &str, _value: &Value) -> Result<(), String> {
        self.fail(())
    }
    async fn fs_dirs(&self, _path: &str, _node: Option<String>) -> Result<Vec<String>, String> {
        self.fail(Vec::new())
    }
    async fn push_vapid(&self) -> Result<String, String> {
        self.fail("publicKey".to_string())
    }
    async fn push_subscribe(&self, _sub: &PushSubscriptionDto) -> Result<(), String> {
        self.fail(())
    }
    async fn push_unsubscribe(&self, _endpoint: &str) -> Result<(), String> {
        self.fail(())
    }
}
