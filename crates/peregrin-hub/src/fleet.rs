//! Fleet dispatch (PL-007 C4) — hub-side runner selection for
//! `kind="campaign"` entities.
//!
//! The entity plane pins a campaign entity to the node that created
//! it; `campaign.start` ops then route to that owner and the node's
//! `runs_on` gate (`CampaignEntityPlane::launch`) 409s a mismatch.
//! Nothing picked *which* node should own the entity — this module
//! closes that gap at the narrowest correct point: [`FleetDispatchApi`]
//! wraps the hub's [`EntityApi`] and, on an **unpinned**
//! `kind="campaign"` create whose init doc declares `runs_on` labels,
//! chooses an eligible node from the sync projection's advertised
//! capabilities (`NodeStatusDto::capabilities`, fed by
//! `PEREGRIN_RUNNER_LABELS`) and passes it to the inner create as the
//! `node` pin. Everything else delegates verbatim — explicit
//! `?node=`/`node` pins, other kinds, and label-less campaign creates
//! keep today's primary-node behavior, and `campaign.*` ops on an
//! already-pinned entity still hit the node's 409 gate unchanged.
//!
//! ## Pick policy (v1 — deterministic, no load awareness)
//!
//! [`pick_runner`] filters projected nodes to those the projection
//! doesn't report `down` whose `capabilities` satisfy `runs_on`
//! (AND semantics via [`peregrin_core::runs_on_satisfied`]), then
//! takes the first after sorting by `(ephemeral, id)`: **non-ephemeral
//! (durable) nodes are preferred, ephemeral runners are fallback
//! capacity**, node id breaks ties. Rationale: an entity pinned to a
//! durable node survives runner churn; `hypha`'s ephemeral reaping
//! (`PEREGRIN_EPHEMERAL_DEAD_AFTER`) drops a dead runner's entities
//! and dead-letters its queued ops — fine for burst CI capacity, bad
//! as the default placement. There is no load/busy awareness yet —
//! revisit when fleets need it. Ops submitted while the picked node
//! is briefly unreachable still queue in the durable outbox, so a
//! transient `down` flap between pick and delivery is safe; a
//! permanently dead pick surfaces as a dead-letter, same as any
//! pinned entity today.
//!
//! ## Where `runs_on` is read from
//!
//! The hub never parses campaign YAML — the create's `indexDoc` (the
//! `init` doc `POST /api/entities` forwards) declares labels either
//! explicitly or inside an embedded JSON campaign doc:
//!
//! ```json
//! { "kind": "campaign", "indexDoc": { "name": "ci-gate",
//!     "runsOn": ["runner:linux", "gpu"] } }
//! { "kind": "campaign", "indexDoc": { "name": "ci-gate",
//!     "campaign": { "runs_on": ["runner:linux"] } } }
//! ```
//!
//! Top-level `runsOn`/`runs_on` wins when both are present. A
//! `campaign.yaml`/`campaign.path` string form isn't inspected —
//! pass `runsOn` or a JSON campaign object. A create with no
//! declared labels is *not* dispatched (primary node, as before);
//! the picker's empty-`runs_on` arm exists for callers that want
//! "any node, deterministically" and for tests.
//!
//! ## Remaining seams
//!
//! - `campaign.start`/`resume` ops on an entity already pinned to an
//!   ineligible node still 409 — moving a live entity is snapshot
//!   export/import territory, out of v1 scope.
//! - `snapshot_import` doesn't dispatch (import target stays
//!   explicit/`?node=`).
//! - No re-dispatch on node death; a dead pick's queued ops
//!   dead-letter per existing outbox semantics.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use hypha_node_api::api::EntityApi;
use hypha_node_api::dto::{
    CreateResultDto, EntityDto, EntityIndexDto, EntityOp, NodeInfoDto, NodeStatusDto, PendingOpDto,
    PushSubscriptionDto,
};
use peregrin_core::runs_on_satisfied;
use serde_json::Value;

/// The campaign entity kind tag — mirrors
/// `peregrin_node::campaign_entity::CAMPAIGN_KIND`; the hub names the
/// wire tag, never the node crate's type.
const CAMPAIGN_KIND: &str = "campaign";

/// `pick_runner` failure — no projected node satisfies `runs_on`.
/// Carries the requirement so the caller's error names what was
/// asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoEligibleRunner {
    /// The `runs_on` labels nothing satisfied.
    pub runs_on: Vec<String>,
}

impl fmt::Display for NoEligibleRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no eligible runner for runs_on {:?} — no projected node advertises every label",
            self.runs_on
        )
    }
}

impl std::error::Error for NoEligibleRunner {}

/// Choose the node a `runs_on`-constrained campaign entity should be
/// pinned to. See the module docs for the policy: non-down nodes
/// whose advertised `capabilities` satisfy every label, durable
/// (non-`ephemeral`) first, node id ascending — fully deterministic.
///
/// # Errors
/// [`NoEligibleRunner`] when no candidate qualifies (including the
/// empty fleet / all-down cases).
pub fn pick_runner(
    runs_on: &[String],
    nodes: &[NodeStatusDto],
) -> Result<String, NoEligibleRunner> {
    let mut eligible: Vec<&NodeStatusDto> = nodes
        .iter()
        .filter(|n| n.status != "down")
        .filter(|n| runs_on_satisfied(runs_on, &n.capabilities))
        .collect();
    // Sort by (ephemeral, id): `false < true` puts durable nodes
    // first; id ascending is the deterministic tiebreak.
    eligible.sort_by(|a, b| a.ephemeral.cmp(&b.ephemeral).then_with(|| a.id.cmp(&b.id)));
    eligible
        .first()
        .map(|n| n.id.clone())
        .ok_or_else(|| NoEligibleRunner {
            runs_on: runs_on.to_vec(),
        })
}

/// The `runs_on` labels a campaign-entity create declares — see the
/// module docs for the contract. Top-level `runsOn`/`runs_on` on the
/// init doc wins; otherwise an embedded JSON `campaign` object's
/// `runs_on`/`runsOn` is read. Non-array/mistyped values are ignored
/// (the node's own `runs_on` gate still applies to whatever campaign
/// the start op carries — a bad hint fails loudly there, not here).
fn declared_runs_on(init: &Value) -> Vec<String> {
    fn labels(v: Option<&Value>) -> Vec<String> {
        v.and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
    let explicit = labels(init.get("runsOn"))
        .into_iter()
        .chain(labels(init.get("runs_on")));
    let mut out: Vec<String> = explicit.collect();
    if out.is_empty() {
        if let Some(campaign) = init.get("campaign") {
            out = labels(campaign.get("runsOn"))
                .into_iter()
                .chain(labels(campaign.get("runs_on")))
                .collect();
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// An [`EntityApi`] decorator that dispatches unpinned `campaign`
/// creates to a `runs_on`-eligible node — see the module docs. Every
/// other call delegates to the wrapped api untouched.
pub struct FleetDispatchApi {
    inner: Arc<dyn EntityApi>,
}

impl FleetDispatchApi {
    /// Wrap `inner` (the hub's [`hypha_hub::sync_api::SyncEntityApi`]
    /// in production).
    #[must_use]
    pub fn new(inner: Arc<dyn EntityApi>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl EntityApi for FleetDispatchApi {
    async fn list_entities(&self, kind: Option<&str>) -> Result<Vec<EntityDto>, String> {
        self.inner.list_entities(kind).await
    }

    async fn entity(&self, id: &str) -> Result<EntityDto, String> {
        self.inner.entity(id).await
    }

    async fn create_entity(
        &self,
        kind: &str,
        init: &Value,
        node: Option<&str>,
    ) -> Result<CreateResultDto, String> {
        // Explicit pins win — `Some("")` reads as unpinned, matching
        // `SyncEntityApi::scoped_client`.
        let pinned = node.is_some_and(|n| !n.is_empty());
        if pinned || kind != CAMPAIGN_KIND {
            return self.inner.create_entity(kind, init, node).await;
        }
        let runs_on = declared_runs_on(init);
        if runs_on.is_empty() {
            return self.inner.create_entity(kind, init, node).await;
        }
        let nodes = self.inner.node_status().await?;
        let picked = pick_runner(&runs_on, &nodes).map_err(|e| e.to_string())?;
        tracing::info!(
            kind = CAMPAIGN_KIND,
            node = %picked,
            runs_on = ?runs_on,
            "fleet dispatch: pinned campaign entity to eligible node"
        );
        self.inner.create_entity(kind, init, Some(&picked)).await
    }

    async fn submit_op(&self, id: &str, op: &EntityOp) -> Result<(), String> {
        self.inner.submit_op(id, op).await
    }

    async fn delete_entity(&self, id: &str) -> Result<(), String> {
        self.inner.delete_entity(id).await
    }

    async fn snapshot_export(&self, id: &str) -> Result<Value, String> {
        self.inner.snapshot_export(id).await
    }

    async fn snapshot_import(&self, body: &Value, node: Option<&str>) -> Result<EntityDto, String> {
        self.inner.snapshot_import(body, node).await
    }

    async fn entity_index(&self) -> Result<Vec<EntityIndexDto>, String> {
        self.inner.entity_index().await
    }

    async fn pending_ops(&self) -> Result<Vec<PendingOpDto>, String> {
        self.inner.pending_ops().await
    }

    async fn node_info(&self) -> Result<NodeInfoDto, String> {
        self.inner.node_info().await
    }

    async fn rename_node(&self, name: &str, node: Option<&str>) -> Result<NodeInfoDto, String> {
        self.inner.rename_node(name, node).await
    }

    async fn node_status(&self) -> Result<Vec<NodeStatusDto>, String> {
        self.inner.node_status().await
    }

    async fn pair(&self, code: &str, node: Option<&str>) -> Result<String, String> {
        self.inner.pair(code, node).await
    }

    async fn get_config(&self) -> Result<BTreeMap<String, Value>, String> {
        self.inner.get_config().await
    }

    async fn set_config(&self, key: &str, value: &Value) -> Result<(), String> {
        self.inner.set_config(key, value).await
    }

    async fn fs_dirs(&self, path: &str, node: Option<String>) -> Result<Vec<String>, String> {
        self.inner.fs_dirs(path, node).await
    }

    async fn push_vapid(&self) -> Result<String, String> {
        self.inner.push_vapid().await
    }

    async fn push_subscribe(&self, subscription: &PushSubscriptionDto) -> Result<(), String> {
        self.inner.push_subscribe(subscription).await
    }

    async fn push_unsubscribe(&self, endpoint: &str) -> Result<(), String> {
        self.inner.push_unsubscribe(endpoint).await
    }

    fn default_entity_kind(&self) -> Option<&'static str> {
        self.inner.default_entity_kind()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::assert_is_empty)]

    use std::sync::Mutex;

    use serde_json::json;

    use super::*;

    fn node(id: &str, caps: &[&str], ephemeral: bool, status: &str) -> NodeStatusDto {
        NodeStatusDto {
            id: id.into(),
            status: status.into(),
            ephemeral,
            capabilities: caps.iter().map(|c| (*c).to_string()).collect(),
            ..NodeStatusDto::default()
        }
    }

    fn labels(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn empty_runs_on_matches_any_up_node() {
        let nodes = vec![node("b", &[], false, "up"), node("a", &[], false, "up")];
        // Empty requirements: every non-down node qualifies; durable
        // first then id → "a".
        assert_eq!(pick_runner(&[], &nodes).unwrap(), "a");
    }

    #[test]
    fn and_semantics_require_every_label() {
        let nodes = vec![
            node("gpu-box", &["runner:linux", "gpu"], false, "up"),
            node("cpu-box", &["runner:linux"], false, "up"),
            node("mac", &["runner:mac", "gpu"], false, "up"),
        ];
        assert_eq!(
            pick_runner(&labels(&["runner:linux", "gpu"]), &nodes).unwrap(),
            "gpu-box"
        );
        assert_eq!(pick_runner(&labels(&["gpu"]), &nodes).unwrap(), "gpu-box");
    }

    #[test]
    fn no_eligible_node_is_a_clear_error() {
        let nodes = vec![node("cpu", &["runner:linux"], false, "up")];
        let err = pick_runner(&labels(&["gpu"]), &nodes).unwrap_err();
        assert_eq!(err.runs_on, labels(&["gpu"]));
        assert!(err.to_string().contains("gpu"));
        // Empty fleet errors too.
        assert!(pick_runner(&labels(&["gpu"]), &[]).is_err());
    }

    #[test]
    fn deterministic_tiebreak_sorted_first() {
        // Two durable eligibles → lowest id wins regardless of input
        // order; an ephemeral eligible never beats a durable one.
        let nodes = vec![
            node("zeta", &["runner:linux"], false, "up"),
            node("alpha", &["runner:linux"], false, "up"),
            node("ephemeral-0", &["runner:linux"], true, "up"),
        ];
        assert_eq!(
            pick_runner(&labels(&["runner:linux"]), &nodes).unwrap(),
            "alpha"
        );
        // All-ephemeral fleet → still deterministic by id.
        let fleet = vec![
            node("e2", &["runner:linux"], true, "up"),
            node("e1", &["runner:linux"], true, "up"),
        ];
        assert_eq!(
            pick_runner(&labels(&["runner:linux"]), &fleet).unwrap(),
            "e1"
        );
    }

    #[test]
    fn down_nodes_are_not_eligible() {
        let nodes = vec![
            node("dead", &["runner:linux", "gpu"], false, "down"),
            node("alive", &["runner:linux"], false, "up"),
        ];
        // `dead` satisfies the labels but is down → `alive` can't
        // satisfy [linux, gpu] either → error, not a dead pick.
        assert!(pick_runner(&labels(&["runner:linux", "gpu"]), &nodes).is_err());
        assert_eq!(
            pick_runner(&labels(&["runner:linux"]), &nodes).unwrap(),
            "alive"
        );
    }

    #[test]
    fn declared_runs_on_sources() {
        // Explicit top-level labels (either spelling).
        assert_eq!(
            declared_runs_on(&json!({"runsOn": ["gpu", "runner:linux"]})),
            labels(&["gpu", "runner:linux"])
        );
        assert_eq!(
            declared_runs_on(&json!({"runs_on": ["gpu"]})),
            labels(&["gpu"])
        );
        // Embedded JSON campaign doc.
        assert_eq!(
            declared_runs_on(&json!({"campaign": {"runs_on": ["runner:linux"]}})),
            labels(&["runner:linux"])
        );
        // Top-level wins over the embedded doc.
        assert_eq!(
            declared_runs_on(&json!({
                "runsOn": ["gpu"],
                "campaign": {"runs_on": ["runner:linux"]}
            })),
            labels(&["gpu"])
        );
        // yaml/path string forms and junk contribute nothing.
        assert!(
            declared_runs_on(&json!({"campaign": {"yaml": "name: x\nruns_on: [gpu]\n"}}))
                .is_empty()
        );
        assert!(declared_runs_on(&json!({"runsOn": "gpu"})).is_empty());
        assert!(declared_runs_on(&json!({"name": "x"})).is_empty());
    }

    /// Recording stub — canned `node_status`, captures the `node` arg
    /// of every `create_entity`; everything else delegates to a
    /// [`FakeEntityApi`] fixture.
    struct StubApi {
        inner: hypha_node_api::testing::FakeEntityApi,
        nodes: Vec<NodeStatusDto>,
        creates: Mutex<Vec<(String, Option<String>)>>,
    }

    impl StubApi {
        fn new(nodes: Vec<NodeStatusDto>) -> Self {
            Self {
                inner: hypha_node_api::testing::FakeEntityApi::seeded(),
                nodes,
                creates: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl EntityApi for StubApi {
        async fn list_entities(&self, kind: Option<&str>) -> Result<Vec<EntityDto>, String> {
            self.inner.list_entities(kind).await
        }
        async fn entity(&self, id: &str) -> Result<EntityDto, String> {
            self.inner.entity(id).await
        }
        async fn create_entity(
            &self,
            kind: &str,
            init: &Value,
            node: Option<&str>,
        ) -> Result<CreateResultDto, String> {
            self.creates
                .lock()
                .unwrap()
                .push((kind.to_string(), node.map(str::to_string)));
            self.inner.create_entity(kind, init, node).await
        }
        async fn submit_op(&self, id: &str, op: &EntityOp) -> Result<(), String> {
            self.inner.submit_op(id, op).await
        }
        async fn delete_entity(&self, id: &str) -> Result<(), String> {
            self.inner.delete_entity(id).await
        }
        async fn snapshot_export(&self, id: &str) -> Result<Value, String> {
            self.inner.snapshot_export(id).await
        }
        async fn snapshot_import(
            &self,
            body: &Value,
            node: Option<&str>,
        ) -> Result<EntityDto, String> {
            self.inner.snapshot_import(body, node).await
        }
        async fn entity_index(&self) -> Result<Vec<EntityIndexDto>, String> {
            self.inner.entity_index().await
        }
        async fn pending_ops(&self) -> Result<Vec<PendingOpDto>, String> {
            self.inner.pending_ops().await
        }
        async fn node_info(&self) -> Result<NodeInfoDto, String> {
            self.inner.node_info().await
        }
        async fn rename_node(&self, name: &str, node: Option<&str>) -> Result<NodeInfoDto, String> {
            self.inner.rename_node(name, node).await
        }
        async fn node_status(&self) -> Result<Vec<NodeStatusDto>, String> {
            Ok(self.nodes.clone())
        }
        async fn pair(&self, code: &str, node: Option<&str>) -> Result<String, String> {
            self.inner.pair(code, node).await
        }
        async fn get_config(&self) -> Result<BTreeMap<String, Value>, String> {
            self.inner.get_config().await
        }
        async fn set_config(&self, key: &str, value: &Value) -> Result<(), String> {
            self.inner.set_config(key, value).await
        }
        async fn fs_dirs(&self, path: &str, node: Option<String>) -> Result<Vec<String>, String> {
            self.inner.fs_dirs(path, node).await
        }
        async fn push_vapid(&self) -> Result<String, String> {
            self.inner.push_vapid().await
        }
        async fn push_subscribe(&self, subscription: &PushSubscriptionDto) -> Result<(), String> {
            self.inner.push_subscribe(subscription).await
        }
        async fn push_unsubscribe(&self, endpoint: &str) -> Result<(), String> {
            self.inner.push_unsubscribe(endpoint).await
        }
        fn default_entity_kind(&self) -> Option<&'static str> {
            self.inner.default_entity_kind()
        }
    }

    fn fleet(nodes: Vec<NodeStatusDto>) -> (FleetDispatchApi, Arc<StubApi>) {
        let stub = Arc::new(StubApi::new(nodes));
        (
            FleetDispatchApi::new(Arc::clone(&stub) as Arc<dyn EntityApi>),
            stub,
        )
    }

    #[tokio::test]
    async fn unpinned_campaign_create_dispatches_to_eligible_node() {
        let (api, stub) = fleet(vec![
            node("hub", &["runner:linux"], false, "up"),
            node("gpu-1", &["runner:linux", "gpu"], true, "up"),
        ]);
        let res = api
            .create_entity(
                "campaign",
                &json!({"name": "train", "runsOn": ["runner:linux", "gpu"]}),
                None,
            )
            .await
            .unwrap();
        assert_eq!(res.id, "e-new");
        let creates = stub.creates.lock().unwrap();
        assert_eq!(
            creates.as_slice(),
            &[("campaign".to_string(), Some("gpu-1".to_string()))]
        );
    }

    #[tokio::test]
    async fn explicit_pin_and_other_kinds_pass_through() {
        let (api, stub) = fleet(vec![node("gpu-1", &["gpu"], false, "up")]);
        // Explicit `?node=` pin wins — no dispatch.
        api.create_entity("campaign", &json!({"runsOn": ["gpu"]}), Some("pinned"))
            .await
            .unwrap();
        // Non-campaign kinds never dispatch.
        api.create_entity("sepia-session", &json!({"runsOn": ["gpu"]}), None)
            .await
            .unwrap();
        // Label-less campaign creates keep primary-node behavior.
        api.create_entity("campaign", &json!({"name": "x"}), None)
            .await
            .unwrap();
        let creates = stub.creates.lock().unwrap();
        assert_eq!(
            creates.as_slice(),
            &[
                ("campaign".to_string(), Some("pinned".to_string())),
                ("sepia-session".to_string(), None),
                ("campaign".to_string(), None),
            ]
        );
    }

    #[tokio::test]
    async fn no_eligible_runner_surfaces_error() {
        let (api, stub) = fleet(vec![node("cpu", &["runner:linux"], false, "up")]);
        let err = api
            .create_entity("campaign", &json!({"runsOn": ["gpu"]}), None)
            .await
            .unwrap_err();
        assert!(err.contains("gpu"));
        assert!(stub.creates.lock().unwrap().is_empty());
    }
}
