//! Wire DTOs for the entity plane — kind-agnostic by design (ADR-0091).
//! `index_doc`/`payload` are opaque JSON to peregrin: the *integration*
//! (e.g. `sepia-peregrin`) defines what they hold for its entity kind.
//! Tolerant decode everywhere (`#[serde(default)]`, `Value` for blobs).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `GET /api/entities` row — one entity as the node/index projects it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDto {
    pub id: String,
    /// Entity kind tag — `sepia-session`, `campaign`, … integration-defined.
    pub kind: String,
    /// Owning node id on multi-node setups (empty → unknown/local).
    #[serde(default)]
    pub node: Option<String>,
    /// The integration's list-view document — title, timestamps,
    /// grouping keys, flags. Opaque to peregrin.
    #[serde(default)]
    pub index_doc: Value,
    /// Another process holds this entity's lock.
    #[serde(default)]
    pub locked: bool,
    /// A live driver/agent is attached.
    #[serde(default)]
    pub live: bool,
    #[serde(default)]
    pub updated_at: String,
}

/// A write targeted at an entity — `kind` is `<entity-kind>.<verb>`
/// namespaced (`sepia.prompt`, `campaign.signal`); `payload` is the
/// integration's wire body, opaque to the substrate.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityOp {
    pub kind: String,
    #[serde(default)]
    pub payload: Value,
}

/// `GET /api/entity-index` — one node's entity index (per-node row on the hub).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityIndexDto {
    pub node: String,
    #[serde(default)]
    pub entities: Vec<EntityDto>,
}

/// `POST /api/entities` → `{id}` + echo of the registered entity.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResultDto {
    pub id: String,
    #[serde(default)]
    pub entity: Option<EntityDto>,
}

/// Queued (`"queued"`) and dead-lettered (`"failed"`) ops in a hub's
/// durable outbox — the multi-node surface for "the node is down, the
/// write replays on reconnect". Direct connections return `[]`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingOpDto {
    pub id: String,
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub enqueued_at: String,
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub last_error: Option<String>,
}

/// `GET /api/node` — the node descriptor.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoDto {
    pub id: String,
    #[serde(default)]
    pub name: String,
}

/// Per-node health row — `node_status` on a hub (projection row), or a
/// synthesized single-row probe on a direct connection.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatusDto {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub label: String,
    /// `"up"`, `"down"`, or `"unknown"` — the projection's reachability.
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub last_seen_at: Option<String>,
}

/// `POST /api/push/subscribe` — the keys sub-object.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushKeysDto {
    pub auth: String,
    pub p256dh: String,
}

/// `PushSubscriptionDto.prefs` — `peregrin_push::PushPrefs` on the wire
/// (both kinds default to on node-side).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushPrefsDto {
    /// The flagged event finished (or errored).
    pub done: bool,
    /// An entity is waiting on a decision.
    pub permission: bool,
}

/// `POST /api/push/subscribe` — `{endpoint, keys, prefs?}`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSubscriptionDto {
    pub endpoint: String,
    pub keys: PushKeysDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefs: Option<PushPrefsDto>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn pending_op_serializes_with_wire_names() {
        let dto = PendingOpDto {
            id: "w1".into(),
            node_id: "tower".into(),
            entity_id: "s1".into(),
            kind: "sepia.prompt".into(),
            status: "failed".into(),
            enqueued_at: "2026-10-08T06:40:34.123Z".into(),
            attempts: 5,
            last_error: Some("node unreachable".into()),
        };
        let v = serde_json::to_value(&dto).unwrap_or_default();
        assert_eq!(v["nodeId"], "tower");
        assert_eq!(v["entityId"], "s1");
        assert_eq!(v["enqueuedAt"], "2026-10-08T06:40:34.123Z");
        // Tolerant decode — a missing field defaults, extra fields drop.
        let parsed: PendingOpDto =
            serde_json::from_str(r#"{"id":"w2","kind":"x.y","extra":1}"#).unwrap_or_default();
        assert_eq!(parsed.id, "w2");
        assert_eq!(parsed.attempts, 0);
    }

    #[test]
    fn entity_index_doc_is_opaque() {
        let e = EntityDto {
            id: "s1".into(),
            kind: "sepia-session".into(),
            index_doc: serde_json::json!({"title": "T", "cwd": "/x"}),
            ..EntityDto::default()
        };
        let back: EntityDto = serde_json::from_value(serde_json::to_value(&e).unwrap()).unwrap();
        assert_eq!(back.index_doc["title"], "T");
    }
}
