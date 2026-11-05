//! Server functions — the browser never talks to the node directly.
//! Each `#[server]` fn runs on the hub (`ssr` builds: inline during SSR,
//! as an HTTP endpoint under `/hub/*` when called from wasm) and calls
//! the node API through the [`NodeApi`] port. `peregrin-hub` provides the
//! client as context; unit tests substitute a stub.

// The `#[server]` macro's client-side trait impl generates `async` fns
// with no `.await` — not ours to fix.
#![allow(clippy::unused_async_trait_impl)]

use std::collections::BTreeMap;

use leptos::prelude::*;
use serde_json::Value;

// Re-exported so `NodeApi` implementors (the hub's `SyncNodeApi`,
// test stubs) name the selector type without a web-core dependency.
// The port + the ureq client live in `peregrin-node-api` — the hub
// depends on them without the leptos stack.
pub use peregrin_node_api::api::RewindSelector;
#[cfg(feature = "ssr")]
pub use peregrin_node_api::api::{HttpNodeApi, NodeApi};

use crate::dto::{
    AgentDto, AttachResultDto, CheckpointDto, CreateResultDto, HistoryPageDto, ImportResultDto,
    NodeInfoDto, NodeStatusDto, PendingWriteDto, ProjectDto, ProjectTransferDto,
    PushSubscriptionDto, RewindResultDto, SessionSummaryDto, TransferResultDto,
};

// leptos-fetch query scopes — nameable fns so mutations can call
// `QueryClient::invalidate_query(scope, &key)` to refresh every
// component sharing the key (the SSE feed also invalidates these).
pub async fn sessions_scope((): ()) -> Result<Vec<SessionSummaryDto>, ServerFnError> {
    list_sessions().await
}
pub async fn pending_scope((): ()) -> Result<Vec<PendingWriteDto>, ServerFnError> {
    pending_writes().await
}
pub async fn agents_scope((): ()) -> Result<Vec<AgentDto>, ServerFnError> {
    list_agents().await
}
pub async fn projects_scope((): ()) -> Result<Vec<ProjectDto>, ServerFnError> {
    list_projects().await
}
pub async fn node_info_scope((): ()) -> Result<NodeInfoDto, ServerFnError> {
    node_info().await
}
pub async fn node_status_scope((): ()) -> Result<Vec<NodeStatusDto>, ServerFnError> {
    node_status().await
}
pub async fn session_scope(
    key: (String, Option<String>),
) -> Result<SessionSummaryDto, ServerFnError> {
    get_session(key.0, key.1).await
}
pub async fn checkpoints_scope(
    (open, id, agent): (bool, String, Option<String>),
) -> Result<Vec<CheckpointDto>, ServerFnError> {
    if !open || id.is_empty() {
        return Ok(Vec::new());
    }
    list_checkpoints(id, agent).await
}
pub async fn history_scope(key: (String, Option<String>)) -> Result<HistoryPageDto, ServerFnError> {
    // Matches the detail panel's page size.
    session_history(key.0, key.1, None, Some(100)).await
}

/// The shared async cache — `QueryClient` provided once in `App`.
/// Every page reads through it: `client.resource(scope, key)`.
pub fn query_client() -> leptos_fetch::QueryClient {
    leptos::prelude::expect_context::<leptos_fetch::QueryClient>()
}

/// App-level shared query handles, created once outside `<Routes>`.
///
/// Per-page `client.resource()` handles share the scope's subscriber
/// walk — a route swap disposes the old page's resource while the new
/// page's attach runs `invalidate_queries_inner`, touching the
/// disposed signal → wasm trap (no panic message; it unwinds inside a
/// callback). Handles hoisted here live under `App`'s owner forever,
/// so the swap races disappear; pages borrow them via
/// `expect_context::<SharedResources>()`.
#[derive(Clone, Copy)]
pub struct SharedResources {
    pub sessions: Resource<Result<Vec<SessionSummaryDto>, ServerFnError>>,
    pub pending: Resource<Result<Vec<PendingWriteDto>, ServerFnError>>,
    pub nodes: Resource<Result<Vec<NodeStatusDto>, ServerFnError>>,
    pub agents: Resource<Result<Vec<AgentDto>, ServerFnError>>,
    pub projects: Resource<Result<Vec<ProjectDto>, ServerFnError>>,
    pub info: Resource<Result<NodeInfoDto, ServerFnError>>,
}

/// Build + provide the shared handles. `App` calls it right after
/// `QueryClient::new().provide()` — unconditional on both targets so
/// SSR resource indexes line up with hydrate's.
pub fn provide_shared_resources() {
    let client = query_client();
    provide_context(SharedResources {
        sessions: client.resource(sessions_scope, || ()),
        pending: client.resource(pending_scope, || ()),
        nodes: client.resource(node_status_scope, || ()),
        agents: client.resource(agents_scope, || ()),
        projects: client.resource(projects_scope, || ()),
        info: client.resource(node_info_scope, || ()),
    });
}

/// The shared handles — `expect_context` for the lazy name.
pub fn shared() -> SharedResources {
    leptos::prelude::expect_context::<SharedResources>()
}

#[cfg(feature = "ssr")]
fn node_api() -> Result<std::sync::Arc<dyn NodeApi>, ServerFnError> {
    use_context::<std::sync::Arc<dyn NodeApi>>()
        .ok_or_else(|| ServerFnError::new("hub: node API client missing from context"))
}

/// `GET /api/sessions` — every session the node can see.
#[server(prefix = "/hub")]
pub async fn list_sessions() -> Result<Vec<SessionSummaryDto>, ServerFnError> {
    node_api()?
        .list_sessions()
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/sessions/{id}/history` — one page. `before` is the node
/// index the next page should end before (the previous page's `start`).
#[server(prefix = "/hub")]
pub async fn session_history(
    session_id: String,
    agent: Option<String>,
    before: Option<i64>,
    limit: Option<i64>,
) -> Result<HistoryPageDto, ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .history(
            &session_id,
            agent.as_deref(),
            before,
            limit.and_then(|l| usize::try_from(l).ok()),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/sessions/{id}` — the summary row for one session.
#[server(prefix = "/hub")]
pub async fn get_session(
    session_id: String,
    agent: Option<String>,
) -> Result<SessionSummaryDto, ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .get_session(&session_id, agent.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/prompt` — `{text?, attachments?}`; at
/// least one of the two must carry content (the node 400s an empty
/// prompt). `attachments` are validated ACP content blocks built by
/// `peregrin_web_core::composer::attachments_payload`.
#[server(prefix = "/hub")]
pub async fn send_prompt(
    session_id: String,
    agent: Option<String>,
    text: String,
    attachments: Vec<Value>,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() || (text.trim().is_empty() && attachments.is_empty()) {
        return Err(ServerFnError::new(
            "a session id and either text or attachments are required",
        ));
    }
    node_api()?
        .prompt(&session_id, agent.as_deref(), &text, &attachments)
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/cancel`.
#[server(prefix = "/hub")]
pub async fn cancel_run(session_id: String, agent: Option<String>) -> Result<(), ServerFnError> {
    node_api()?
        .cancel(&session_id, agent.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions` — `{cwd, agent?, title?, model?}`; `node` pins
/// the create to one registered node on a multi-node hub.
#[server(prefix = "/hub")]
pub async fn create_session(
    cwd: String,
    agent: Option<String>,
    title: Option<String>,
    model: Option<String>,
    node: Option<String>,
) -> Result<CreateResultDto, ServerFnError> {
    let cwd = cwd.trim();
    if cwd.is_empty() {
        return Err(ServerFnError::new("cwd is required"));
    }
    let clean = |s: Option<String>| s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    node_api()?
        .create_session(
            cwd,
            clean(agent).as_deref(),
            clean(title).as_deref(),
            clean(model).as_deref(),
            clean(node).as_deref(),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/attach` — `takeover` steals a held lock.
#[server(prefix = "/hub")]
pub async fn attach_session(
    session_id: String,
    agent: Option<String>,
    takeover: bool,
) -> Result<AttachResultDto, ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .attach(&session_id, agent.as_deref(), takeover)
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/detach`.
#[server(prefix = "/hub")]
pub async fn detach_session(
    session_id: String,
    agent: Option<String>,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .detach(&session_id, agent.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/permission` — `{requestId, optionId}`;
/// `option_id` `None` dismisses the request.
#[server(prefix = "/hub")]
pub async fn answer_permission(
    session_id: String,
    agent: Option<String>,
    request_id: String,
    option_id: Option<String>,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() || request_id.is_empty() {
        return Err(ServerFnError::new("session id and request id are required"));
    }
    node_api()?
        .answer_permission(
            &session_id,
            agent.as_deref(),
            &request_id,
            option_id.as_deref(),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `PATCH /api/sessions/{id}` — `{title}` only; the meta overlay takes
/// more, but rename is all the UI needs today.
#[server(prefix = "/hub")]
pub async fn rename_session(
    session_id: String,
    agent: Option<String>,
    title: String,
) -> Result<(), ServerFnError> {
    let title = title.trim();
    if session_id.is_empty() || title.is_empty() || title.len() > 200 {
        return Err(ServerFnError::new(
            "a session id and a non-empty title (max 200) are required",
        ));
    }
    node_api()?
        .patch_meta(
            &session_id,
            agent.as_deref(),
            &serde_json::json!({ "title": title }),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `PATCH /api/sessions/{id}` — `{projectIds}` on the meta overlay;
/// the project-membership editor's write path.
#[server(prefix = "/hub")]
pub async fn set_session_projects(
    session_id: String,
    agent: Option<String>,
    project_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .patch_meta(
            &session_id,
            agent.as_deref(),
            &serde_json::json!({ "projectIds": project_ids }),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `PATCH /api/sessions/{id}` — `{pinned}` on the meta overlay.
#[server(prefix = "/hub")]
pub async fn pin_session(
    session_id: String,
    agent: Option<String>,
    pinned: bool,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .patch_meta(
            &session_id,
            agent.as_deref(),
            &serde_json::json!({ "pinned": pinned }),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `PATCH /api/sessions/{id}` — `{model}` on the meta overlay;
/// `None` clears the override back to the agent default. The node
/// caps model names at 100 chars — checked here to fail cheap.
#[server(prefix = "/hub")]
pub async fn set_session_model(
    session_id: String,
    agent: Option<String>,
    model: Option<String>,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() || model.as_deref().is_some_and(|m| m.len() > 100) {
        return Err(ServerFnError::new(
            "a session id and a model string (max 100) or null are required",
        ));
    }
    let patch = peregrin_web_core::composer::model_patch(model.as_deref());
    node_api()?
        .patch_meta(&session_id, agent.as_deref(), &patch)
        .await
        .map_err(ServerFnError::new)
}

/// `PATCH /api/sessions/{id}` — `{archived}` on the meta overlay.
#[server(prefix = "/hub")]
pub async fn archive_session(
    session_id: String,
    agent: Option<String>,
    archived: bool,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .patch_meta(
            &session_id,
            agent.as_deref(),
            &serde_json::json!({ "archived": archived }),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `DELETE /api/sessions/{id}`.
#[server(prefix = "/hub")]
pub async fn delete_session(
    session_id: String,
    agent: Option<String>,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .delete_session(&session_id, agent.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/sessions/{id}/checkpoints`.
#[server(prefix = "/hub")]
pub async fn list_checkpoints(
    session_id: String,
    agent: Option<String>,
) -> Result<Vec<CheckpointDto>, ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .checkpoints(&session_id, agent.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/restore` — `{confirm: true, checkpoint}`.
#[server(prefix = "/hub")]
pub async fn restore_checkpoint(
    session_id: String,
    agent: Option<String>,
    checkpoint: String,
) -> Result<(), ServerFnError> {
    if session_id.is_empty() || checkpoint.is_empty() {
        return Err(ServerFnError::new("session id and checkpoint are required"));
    }
    node_api()?
        .restore(&session_id, agent.as_deref(), &checkpoint)
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/rewind` — `{confirm: true}` + the selector
/// (`RewindSelector::NodeId` from the per-message action,
/// `::Checkpoint` from the details sheet's checkpoint rows).
#[server(prefix = "/hub")]
pub async fn rewind_session(
    session_id: String,
    agent: Option<String>,
    selector: RewindSelector,
) -> Result<RewindResultDto, ServerFnError> {
    if session_id.is_empty() {
        return Err(ServerFnError::new("session id is required"));
    }
    node_api()?
        .rewind(&session_id, agent.as_deref(), &selector)
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/{id}/convert` — copy the session into another
/// agent's store (`{agent: "cline"|"devin"}`); returns the new
/// session's id so the UI can jump to it.
#[server(prefix = "/hub")]
pub async fn convert_session(
    session_id: String,
    agent: Option<String>,
    target: String,
) -> Result<String, ServerFnError> {
    if session_id.is_empty() || !peregrin_web_core::composer::is_convert_target(&target) {
        return Err(ServerFnError::new(
            "a session id and a convert target ('cline' or 'devin') are required",
        ));
    }
    node_api()?
        .convert(&session_id, agent.as_deref(), &target)
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/sessions/import` — `body` is the
/// `composer::import_body` payload (`{agent, session, cwd?}`). Checked
/// again here: the server fn is a public endpoint, the UI's shaping
/// isn't.
#[server(prefix = "/hub")]
pub async fn import_session(body: Value) -> Result<ImportResultDto, ServerFnError> {
    let agent_ok = body
        .get("agent")
        .and_then(Value::as_str)
        .is_some_and(peregrin_web_core::composer::is_convert_target);
    if !body.get("session").is_some_and(Value::is_object) || !agent_ok {
        return Err(ServerFnError::new(
            "import requires {agent: 'cline'|'devin', session: {…IR…}}",
        ));
    }
    node_api()?.import(&body).await.map_err(ServerFnError::new)
}

/// Cross-node transfer (`docs/transfer.md`): the hub exports the IR
/// off the session's node and imports it on `target_node`. `cwd`, when
/// non-empty, remaps the working directory on the target machine.
#[server(prefix = "/hub")]
pub async fn transfer_session(
    session_id: String,
    agent: Option<String>,
    target_node: String,
    cwd: Option<String>,
) -> Result<TransferResultDto, ServerFnError> {
    let cwd = cwd.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    if session_id.is_empty() || target_node.trim().is_empty() {
        return Err(ServerFnError::new(
            "a session id and a target node are required",
        ));
    }
    node_api()?
        .transfer_session(
            &session_id,
            agent.as_deref(),
            target_node.trim(),
            cwd.as_deref(),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `transfer_session` over every session under `project_cwd` on
/// `source_node` — the cwd-group move. Per-session failures collect
/// in the result's `failed` list rather than aborting.
#[server(prefix = "/hub")]
pub async fn transfer_project(
    project_cwd: String,
    source_node: String,
    target_node: String,
    cwd: Option<String>,
) -> Result<ProjectTransferDto, ServerFnError> {
    let cwd = cwd.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    if project_cwd.trim().is_empty()
        || source_node.trim().is_empty()
        || target_node.trim().is_empty()
    {
        return Err(ServerFnError::new(
            "a project cwd, a source node, and a target node are required",
        ));
    }
    node_api()?
        .transfer_project(
            project_cwd.trim(),
            source_node.trim(),
            target_node.trim(),
            cwd.as_deref(),
        )
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/agents`.
#[server(prefix = "/hub")]
pub async fn list_agents() -> Result<Vec<AgentDto>, ServerFnError> {
    node_api()?.list_agents().await.map_err(ServerFnError::new)
}

/// `GET /api/projects`.
#[server(prefix = "/hub")]
pub async fn list_projects() -> Result<Vec<ProjectDto>, ServerFnError> {
    node_api()?
        .list_projects()
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/projects` — `{name}`; `node` pins the create to one
/// registered node on a multi-node hub.
#[server(prefix = "/hub")]
pub async fn create_project(
    name: String,
    node: Option<String>,
) -> Result<ProjectDto, ServerFnError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ServerFnError::new(
            "name must be a non-empty string (max 100)",
        ));
    }
    node_api()?
        .create_project(name, node.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `DELETE /api/projects/{id}`.
#[server(prefix = "/hub")]
pub async fn delete_project(project_id: String, node: Option<String>) -> Result<(), ServerFnError> {
    if project_id.is_empty() {
        return Err(ServerFnError::new("project id is required"));
    }
    node_api()?
        .delete_project(&project_id, node.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/config` — the node's public config map.
#[server(prefix = "/hub")]
pub async fn get_config() -> Result<BTreeMap<String, Value>, ServerFnError> {
    node_api()?.get_config().await.map_err(ServerFnError::new)
}

/// `PATCH /api/config/{key}` — `{value}`.
#[server(prefix = "/hub")]
pub async fn set_config(key: String, value: Value) -> Result<(), ServerFnError> {
    if key.trim().is_empty() {
        return Err(ServerFnError::new("config key is required"));
    }
    node_api()?
        .set_config(&key, &value)
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/fs?path=` — subdirectory listing for the cwd picker.
/// `node` picks the node on a multi-node hub (`None` = primary).
#[server(prefix = "/hub")]
pub async fn fs_dirs(path: String, node: Option<String>) -> Result<Vec<String>, ServerFnError> {
    if path.is_empty() {
        return Err(ServerFnError::new("path is required"));
    }
    node_api()?
        .fs_dirs(&path, node)
        .await
        .map_err(ServerFnError::new)
}

/// `POST /api/pair` — redeem a pairing code (from `peregrin pair`) for a
/// node credential. The minted token is returned once — the UI shows
/// it and tells the user to store it.
#[server(prefix = "/hub")]
pub async fn redeem_pair_code(code: String, node: Option<String>) -> Result<String, ServerFnError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(ServerFnError::new("a pairing code is required"));
    }
    node_api()?
        .pair(code, node.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/node` — the primary node's descriptor.
#[server(prefix = "/hub")]
pub async fn node_info() -> Result<NodeInfoDto, ServerFnError> {
    node_api()?.node_info().await.map_err(ServerFnError::new)
}

/// `PATCH /api/node` — `{name}`; `node` picks the target on a
/// multi-node hub (`None` = primary). Returns the updated descriptor.
#[server(prefix = "/hub")]
pub async fn rename_node(name: String, node: Option<String>) -> Result<NodeInfoDto, ServerFnError> {
    let name = name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(ServerFnError::new(
            "name must be a non-empty string (max 100)",
        ));
    }
    node_api()?
        .rename_node(name, node.as_deref())
        .await
        .map_err(ServerFnError::new)
}

/// Per-node health rows — the hub registry + sync engine status.
#[server(prefix = "/hub")]
pub async fn node_status() -> Result<Vec<NodeStatusDto>, ServerFnError> {
    node_api()?.node_status().await.map_err(ServerFnError::new)
}

/// Queued/failed outbox writes across every registered node — `[]` on a
/// single-node hub, where writes fail inline instead of queueing.
#[server(prefix = "/hub")]
pub async fn pending_writes() -> Result<Vec<PendingWriteDto>, ServerFnError> {
    node_api()?
        .pending_writes()
        .await
        .map_err(ServerFnError::new)
}

/// `GET /api/push/vapid` — the public key push subscriptions are made
/// against.
#[server(prefix = "/hub")]
pub async fn push_vapid_key() -> Result<String, ServerFnError> {
    node_api()?.push_vapid().await.map_err(ServerFnError::new)
}

/// `POST /api/push/subscribe` — register a browser push endpoint.
#[server(prefix = "/hub")]
pub async fn push_subscribe(subscription: PushSubscriptionDto) -> Result<(), ServerFnError> {
    if subscription.endpoint.is_empty() {
        return Err(ServerFnError::new("push endpoint is required"));
    }
    node_api()?
        .push_subscribe(&subscription)
        .await
        .map_err(ServerFnError::new)
}

/// `DELETE /api/push/subscribe` — drop a browser push endpoint.
#[server(prefix = "/hub")]
pub async fn push_unsubscribe(endpoint: String) -> Result<(), ServerFnError> {
    if endpoint.is_empty() {
        return Err(ServerFnError::new("push endpoint is required"));
    }
    node_api()?
        .push_unsubscribe(&endpoint)
        .await
        .map_err(ServerFnError::new)
}
