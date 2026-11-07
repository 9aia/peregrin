//! peregrin-driver-subworkflow — `driver: subworkflow`, the callable
//! composition boundary (ADR-0022; the ADR's `handl-driver-subworkflow`
//! package lands here under the `peregrin-*` naming).
//!
//! ```yaml
//! - id: setup-worktree
//!   driver: subworkflow
//!   on: [start]
//!   emits: [worktree.ready, worktree.failed]
//!   params:
//!     path: lib/git/worktree.yaml          # relative to the campaign file's dir
//!     inputs: {repo: "${config.repo}"}     # → the sub-run's config
//!     outputs:                             # sub-event type → parent event type
//!       worktree.ready: worktree.ready
//!       worktree.failed: worktree.failed
//! ```
//!
//! ## Semantics
//!
//! - `params.path` (required) — the campaign YAML to run. Relative
//!   paths resolve against the loading campaign's directory — the same
//!   anchor `include:` uses (`ctx.campaign_dir` ←
//!   [`peregrin_core::Campaign::source_dir`], stamped by the loader) —
//!   falling back to the process cwd for campaigns built
//!   programmatically.
//! - `params.inputs` (optional mapping) — merged over the
//!   sub-campaign's declared `config:` block: inputs win key-by-key,
//!   so declared config entries act as defaults.
//! - `params.outputs` (optional mapping `sub type → parent type`) —
//!   after the sub-run terminates its event log is scanned in order
//!   and the *first* event whose type is an `outputs` key is
//!   re-emitted under the mapped type as this node's result. Engine
//!   terminal events are in the log, so
//!   `outputs: {workflow.failed: worktree.failed}` maps sub-run
//!   failure to a parent event. With no `outputs` (or an empty map),
//!   a successful sub-run emits `emits[0]` (or `{id}.done`) with a
//!   `{sub_run: {...}}` marker payload.
//! - Result payload — an object sub-event payload passes through with
//!   a `sub_run: {type, source}` provenance key merged in; a
//!   non-object payload wraps as `{value, sub_run}`.
//! - Secrets — the sub-run inherits the parent run's secrets record
//!   (`ctx.secrets` cloned into the sub-run's [`SubRunOptions`]), like
//!   a lexical scope.
//! - Failure — a missing `params.path`, an unreadable/unparseable
//!   target, a sub-run ending `workflow.failed`/`workflow.cancelled`
//!   with no matching output, or a sub-run producing none of the
//!   declared outputs is a [`DriverError`] → `node.failed` (the usual
//!   retry machinery applies — a retried node re-runs the
//!   sub-workflow).
//!
//! ## Isolation
//!
//! The sub-run executes on a fresh engine instance via
//! [`ExecutionContext::run_sub`] — own bus, event log, state store and
//! timer heap, over the parent's driver registry minus ingress
//! drivers (an ingress node inside a finite sub-run could only feed
//! the parent's channel: a namespace leak). Nothing the sub-workflow
//! emits reaches the parent namespace except the single mapped output
//! event. `invoke` blocks the sequential walker for the sub-run's
//! duration — the runner has no async spawn; acceptable for v1.
//! `ctx.request_shutdown` inside a sub-run cancels the sub-run, not
//! the parent; a parent `workflow.cancel` mid-invoke does not
//! interrupt a running sub-workflow.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, SubRunOptions};
use peregrin_loader::Loader;
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, SystemEffects,
    resolve_value_opts,
};

/// `emits[0]` — the `driver`-convention fallback (`{id}.done`).
fn emit_kind(node: &Node) -> String {
    node.emits
        .first()
        .cloned()
        .unwrap_or_else(|| format!("{}.done", node.id))
}

/// `params.path` → an fs path: absolute passes through; relative joins
/// the parent campaign's directory (`ctx.campaign_dir`), falling back
/// to the process cwd — the same anchoring `include:` applies.
fn full_path(path: &str, campaign_dir: Option<&Path>) -> PathBuf {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }
    campaign_dir
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        .join(candidate)
}

/// The event this node emits for a matched sub-event: an object
/// payload passes through with a `sub_run: {type, source}` provenance
/// key merged in; a non-object payload wraps as `{value, sub_run}`.
fn mapped_payload(event: &Event) -> Value {
    let meta = serde_json::json!({"type": event.kind, "source": event.source});
    match &event.payload {
        Value::Object(map) => {
            let mut payload = map.clone();
            payload.insert("sub_run".to_string(), meta);
            Value::Object(payload)
        }
        other => serde_json::json!({"value": other.clone(), "sub_run": meta}),
    }
}

fn schema(fields: &[(&str, &str)]) -> EventSchema {
    EventSchema {
        kind: String::new(),
        fields: fields
            .iter()
            .map(|(k, t)| ((*k).to_string(), Value::String((*t).to_string())))
            .collect(),
        schema: Map::new(),
        annotations: Map::new(),
    }
}

/// `driver: subworkflow` — runs another campaign file as a nested,
/// namespaced run and maps one of its terminal events back as this
/// node's result. `loader` loads `params.path` (`YamlLoader` by
/// default); `effects` backs the driver-side param re-resolve the
/// other impls perform inside `invoke`.
pub struct SubworkflowDriver {
    loader: Arc<dyn Loader>,
    effects: Arc<dyn Effects>,
}

impl Default for SubworkflowDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl SubworkflowDriver {
    /// A driver over [`peregrin_loader_yaml::YamlLoader`] +
    /// [`SystemEffects`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            loader: Arc::new(peregrin_loader_yaml::YamlLoader),
            effects: Arc::new(SystemEffects),
        }
    }

    /// A driver over a custom loader (tests, other formats).
    #[must_use]
    pub fn with_loader(loader: Arc<dyn Loader>) -> Self {
        Self {
            loader,
            effects: Arc::new(SystemEffects),
        }
    }

    /// A driver over custom effects (tests).
    #[must_use]
    pub fn with_effects(effects: Arc<dyn Effects>) -> Self {
        Self {
            effects,
            ..Self::new()
        }
    }

    /// Re-resolve `node.params` inside `invoke` the way every impl
    /// does — `event.*` binds the first trigger so `path`/`inputs` can
    /// template off whatever woke the node.
    fn resolve_params(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Map<String, Value>, DriverError> {
        let tctx = Context {
            strict: ctx.strict,
            missing_secrets: ctx.missing_secrets.clone(),
            capabilities: (*ctx.capabilities).clone(),
            effects: &*self.effects,
        };
        let state_view = |key: &str| ctx.state.borrow().get(key);
        let resolved = resolve_value_opts(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &[],
                scope: Scope {
                    event: triggers.first().map(|event| EventScope {
                        event,
                        patterns: &node.on,
                    }),
                    config: Some(&ctx.config),
                    params: ParamsScope::Resolved(&ctx.params),
                    state: Some(&state_view),
                    node: Some(NodeScope {
                        id: &node.id,
                        fires: ctx.fires,
                    }),
                    secrets: Some(&ctx.secrets),
                },
            },
        )
        .map_err(|error| DriverError::new(error.to_string()))?;
        Ok(resolved.as_object().cloned().unwrap_or_default())
    }

    /// `params.outputs` → the `sub type → parent type` map. Absent or
    /// an empty mapping reads as "no declared outputs" (side-effect
    /// call); non-string values are a config error.
    fn outputs_map(
        params: &Map<String, Value>,
        err: &dyn Fn(&str) -> DriverError,
    ) -> Result<Option<Map<String, Value>>, DriverError> {
        let outputs = match params.get("outputs") {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::Object(map)) => map,
            Some(_) => return Err(err("'params.outputs' must be a mapping")),
        };
        if outputs.is_empty() {
            return Ok(None);
        }
        for (sub_type, parent_type) in outputs {
            if parent_type.as_str().is_none_or(str::is_empty) {
                return Err(err(&format!(
                    "'params.outputs.{sub_type}' must be a non-empty event type"
                )));
            }
        }
        Ok(Some(outputs.clone()))
    }

    /// Scan the sub-run's log for the first event whose type is an
    /// `outputs` key and emit it under the mapped parent type. Engine
    /// terminal events sit in the log too, so `workflow.failed` is a
    /// valid key for failure mapping.
    fn map_log(
        node: &Node,
        path: &str,
        log: &[Event],
        outputs: Option<&Map<String, Value>>,
        err: &dyn Fn(&str) -> DriverError,
    ) -> Result<Vec<Event>, DriverError> {
        if let Some(outputs) = outputs {
            for event in log {
                if let Some(parent_type) = outputs.get(&event.kind) {
                    let Some(parent_type) = parent_type.as_str() else {
                        continue;
                    };
                    return Ok(vec![Event::new(
                        parent_type,
                        mapped_payload(event),
                        &node.id,
                    )]);
                }
            }
            let terminal = log.last().map_or("", |e| e.kind.as_str());
            let keys = outputs.keys().cloned().collect::<Vec<_>>().join(", ");
            return Err(err(&match terminal {
                "workflow.failed" | "workflow.cancelled" => format!(
                    "sub-run '{path}' ended with '{terminal}' and none of the declared outputs fired ({keys})"
                ),
                _ => format!("sub-run '{path}' produced none of the declared outputs ({keys})"),
            }));
        }
        // No declared outputs — a side-effect call: success emits the
        // done marker; a failed/cancelled sub-run is a DriverError.
        match log.last().map(|e| e.kind.as_str()) {
            Some("workflow.done") => Ok(vec![Event::new(
                emit_kind(node),
                serde_json::json!({
                    "sub_run": { "path": path, "events": log.len() },
                }),
                &node.id,
            )]),
            Some(terminal) => Err(err(&format!("sub-run '{path}' ended with '{terminal}'"))),
            None => Err(err(&format!("sub-run '{path}' produced an empty log"))),
        }
    }
}

impl Driver for SubworkflowDriver {
    fn name(&self) -> &'static str {
        "subworkflow"
    }

    fn input(&self) -> Option<EventSchema> {
        Some(schema(&[
            ("path", "string"),
            ("inputs", "object?"),
            ("outputs", "object?"),
        ]))
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let err =
            |reason: &str| DriverError::new(format!("subworkflow node '{}': {reason}", node.id));
        let params = self.resolve_params(node, triggers, ctx)?;
        let Some(path) = params
            .get("path")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            return Err(err("missing 'params.path' — the campaign YAML to run"));
        };
        let resolved_path = full_path(path, ctx.campaign_dir.as_deref());
        let mut campaign = self
            .loader
            .load(&resolved_path)
            .map_err(|e| err(&format!("cannot load '{path}': {e}")))?;

        // `inputs` → the sub-run's `config`: merged over the
        // sub-campaign's declared `config:` so declared keys act as
        // defaults and the call site's values win (ADR-0022).
        match params.get("inputs") {
            None | Some(Value::Null) => {}
            Some(Value::Object(inputs)) => {
                for (key, value) in inputs {
                    campaign.config.insert(key.clone(), value.clone());
                }
            }
            Some(_) => return Err(err("'params.inputs' must be a mapping")),
        }

        let outputs = Self::outputs_map(&params, &err)?;
        let log = ctx.run_sub(
            &campaign,
            &SubRunOptions {
                // The sub-run inherits the parent run's secrets record —
                // a lexical-scope read, not a fresh lookup.
                secrets: (*ctx.secrets).clone(),
                dry_run: ctx.dry_run,
            },
        )?;
        Self::map_log(node, path, &log, outputs.as_ref(), &err)
    }
}
