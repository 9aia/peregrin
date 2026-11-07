//! peregrin-driver — the campaign driver port (TS `Driver` +
//! `ExecutionContext`). The engine resolves a driver per node through
//! the registry and calls [`Driver::invoke`] when the node fires.
//! Impls: `peregrin-driver-shell` first; http/agent/human/webhook next.
//!
//! The port is synchronous — the engine runs a sequential,
//! queue-driven executor, and driver-backed template functions must be
//! callable from sync template resolution. Blocking I/O impls (e.g.
//! `ureq`) fit; async impls need their own runtime.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{Map, Value};

use peregrin_core::{Campaign, Event, EventSchema, Node, PythonImport};
use peregrin_store::StateStore;

/// A deferred emission queued through
/// [`ExecutionContext::schedule_at`] — the runner moves it onto its
/// timer heap and publishes `event` once `at` (an [`Instant`]) is
/// reached. This is the `delay`/`schedule` drivers' side channel
/// (ADR-0018): the future event lives in the engine, so `invoke`
/// returns immediately instead of blocking the run thread on a
/// `sleep`. Deferred events hold the run open until they fire; a
/// `workflow.cancel` abandons them, and retiring a node drops its
/// pending emissions.
#[derive(Debug)]
pub struct ScheduledEmit {
    /// The event to publish when the deadline lands.
    pub event: Event,
    /// Fire time — a monotonic clock instant inside the run.
    pub at: Instant,
}

/// Driver failure — the Rust analogue of the TS `DriverError`
/// (`{reason}`). The engine retries any failure up to `max_attempts`;
/// there is no retryable flag in the TS semantics.
#[derive(Debug, thiserror::Error)]
#[error("{reason}")]
pub struct DriverError {
    /// Human-readable reason — the string TS tests assert on.
    pub reason: String,
}

impl DriverError {
    /// Build an error from any reason string.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Options a driver hands the engine for a nested run through
/// [`ExecutionContext::run_sub`] (ADR-0022). The campaign itself is
/// passed fully materialized — the driver already merged its `inputs`
/// into `campaign.config`, so the engine only applies run options.
#[derive(Clone, Default)]
pub struct SubRunOptions {
    /// The secrets record the sub-run sees — the `subworkflow` driver
    /// passes the parent run's record so the nested run inherits the
    /// same secrets (`ctx.secrets` clone), like a lexical scope.
    pub secrets: BTreeMap<String, String>,
    /// Fallback secret stores the sub-run resolves through
    /// ([`ExecutionContext::secret_stores`]) — passed verbatim so the
    /// sub-run's ambient-env/keyring/file legs match the parent's.
    pub secret_stores: Vec<SecretSource>,
    /// Propagated `ctx.dry_run` — the sub-run's drivers are skipped
    /// the same way the parent's are.
    pub dry_run: bool,
}

/// The engine-provided nested-run channel behind
/// [`ExecutionContext::run_sub`] — runs `campaign` on a fresh,
/// isolated engine instance (own bus, event log, state, timers) and
/// returns the sub-run's event log. `Rc`-bound like the rest of the
/// ctx — the engine is single-threaded.
pub type SubRunner = dyn Fn(&Campaign, &SubRunOptions) -> Result<Vec<Event>, DriverError>;

/// The execution context handed to a driver on invoke — port of the
/// TS `ExecutionContext`. Side channels (`emit`, `inject`,
/// `request_shutdown`, `missing_secrets`, `state`) are interior-shared
/// `Rc`s so child contexts (driver-backed template functions) write to
/// the same run state the TS shared object carried.
#[derive(Clone)]
pub struct ExecutionContext {
    /// Workflow `config:` (TS `ctx.config`).
    pub config: Rc<Map<String, Value>>,
    /// The fire's resolved `params:` object — the `${params.*}` binding
    /// for driver-side re-resolves (scope-resolution spec); empty until
    /// the runner fills it post-resolution.
    pub params: Rc<Map<String, Value>>,
    /// The run's secrets record — the explicit leg of the resolution
    /// chain (caller-provided `RunOptions.secrets`, which override
    /// ambient sources). Read via [`ExecutionContext::secret`] /
    /// [`ExecutionContext::get_secret`] /
    /// [`ExecutionContext::resolve_secret`] so misses are audited
    /// (TS `secrets` proxy).
    pub secrets: Rc<BTreeMap<String, String>>,
    /// Fallback secret stores consulted — in order — after the
    /// record and the ambient environment miss (the documented
    /// `record → env → keyring → file` chain's tail). The
    /// composition root wires these (OS keyring, env file); an
    /// empty list keeps record+env only. Resolved values stay in
    /// the ctx — nothing serializes them into run state.
    pub secret_stores: Rc<Vec<SecretSource>>,
    /// Snapshot of the event log at node start (TS `ctx.log`).
    pub log: Rc<Vec<Event>>,
    /// Events pushed via [`ExecutionContext::emit`] — the runner drains
    /// this into the pending queue after each node fire.
    pub emitted: Rc<RefCell<Vec<Event>>>,
    /// Node specs pushed via [`ExecutionContext::inject`] — the runner
    /// feeds them to `bus.inject`.
    pub injected: Rc<RefCell<Vec<Value>>>,
    /// Deferred emissions pushed via [`ExecutionContext::schedule_at`]
    /// — the runner moves them onto its timer heap after each node
    /// fire.
    pub scheduled: Rc<RefCell<Vec<ScheduledEmit>>>,
    /// Set by [`ExecutionContext::request_shutdown`].
    pub shutdown: Rc<Cell<bool>>,
    /// The node's state store — scoped when the node declares
    /// `state.key`, else the shared run store (TS `ctx.state`).
    pub state: Rc<RefCell<Box<dyn StateStore>>>,
    /// `options.dryRun` — drivers should not perform side effects.
    pub dry_run: bool,
    /// `${node.id}:${attempt}` — unique per driver invocation
    /// (TS `ctx.idempotencyKey`).
    pub idempotency_key: String,
    /// W3C traceparent of the node span (TS `ctx.traceparent`).
    pub traceparent: String,
    /// How many times this node has fired, counting this fire
    /// (TS `ctx.fires`, incremented when the bus schedules the node).
    pub fires: u64,
    /// Workflow `strict` flag (TS `ctx.strict`).
    pub strict: bool,
    /// Missing-secret audit list — shared with the template context so
    /// strict-mode checks see driver-side misses (TS `missingSecrets`).
    pub missing_secrets: Rc<RefCell<Vec<String>>>,
    /// `{...workflow.capabilities, ...node.capabilities}` — shallow
    /// merge (TS `ctx.capabilities`).
    pub capabilities: Rc<Map<String, Value>>,
    /// Node `resources:` (TS `ctx.resources`).
    pub resources: Rc<Map<String, Value>>,
    /// Workflow-level `capabilities:` (TS `ctx.workflowCapabilities`).
    pub workflow_capabilities: Rc<Map<String, Value>>,
    /// Workflow `import:` python modules (TS `ctx.pythonImports`).
    pub python_imports: Rc<Vec<PythonImport>>,
    /// Directory the campaign was loaded from (`Campaign::source_dir`)
    /// — the anchor run-time relative paths resolve against
    /// (`subworkflow` `params.path`, ADR-0022), mirroring `include:`.
    /// `None` for campaigns built programmatically; drivers fall back
    /// to the process cwd.
    pub campaign_dir: Option<PathBuf>,
    /// The nested-run channel — set by the engine; `None` on detached
    /// contexts ([`ExecutionContext::default`], driver-backed template
    /// functions run without an enclosing run).
    pub sub_runner: Option<Rc<SubRunner>>,
}

impl Default for ExecutionContext {
    /// A detached context — isolated channels, empty everything.
    /// Used when a driver-backed template function runs without an
    /// enclosing node context.
    fn default() -> Self {
        Self {
            config: Rc::new(Map::new()),
            params: Rc::new(Map::new()),
            secrets: Rc::new(BTreeMap::new()),
            secret_stores: Rc::new(Vec::new()),
            log: Rc::new(Vec::new()),
            emitted: Rc::new(RefCell::new(Vec::new())),
            injected: Rc::new(RefCell::new(Vec::new())),
            scheduled: Rc::new(RefCell::new(Vec::new())),
            shutdown: Rc::new(Cell::new(false)),
            state: Rc::new(RefCell::new(Box::new(
                peregrin_store::MemoryStore::default(),
            ))),
            dry_run: false,
            idempotency_key: String::new(),
            traceparent: String::new(),
            fires: 0,
            strict: false,
            missing_secrets: Rc::new(RefCell::new(Vec::new())),
            capabilities: Rc::new(Map::new()),
            resources: Rc::new(Map::new()),
            workflow_capabilities: Rc::new(Map::new()),
            python_imports: Rc::new(Vec::new()),
            campaign_dir: None,
            sub_runner: None,
        }
    }
}

impl ExecutionContext {
    /// `ctx.emit(event)` — queue an event for the run's pending queue.
    pub fn emit(&self, event: Event) {
        self.emitted.borrow_mut().push(event);
    }

    /// `ctx.inject(nodeSpecs)` — add nodes to the bus mid-run.
    pub fn inject(&self, node_specs: &[Value]) {
        self.injected
            .borrow_mut()
            .extend(node_specs.iter().cloned());
    }

    /// `ctx.schedule_at(event, at)` — defer `event` until `at` (an
    /// [`Instant`]); the runner's timer heap publishes it then
    /// (ADR-0018). Deferred events hold the run open until they fire
    /// — unlike poll ticks, which only land while the run is
    /// otherwise alive.
    pub fn schedule_at(&self, event: Event, at: Instant) {
        self.scheduled
            .borrow_mut()
            .push(ScheduledEmit { event, at });
    }

    /// `ctx.schedule_in(event, after)` — [`ExecutionContext::schedule_at`]
    /// relative to now.
    pub fn schedule_in(&self, event: Event, after: std::time::Duration) {
        self.schedule_at(event, Instant::now() + after);
    }

    /// Drain queued deferred emissions (runner-internal).
    pub fn take_scheduled(&self) -> Vec<ScheduledEmit> {
        std::mem::take(&mut *self.scheduled.borrow_mut())
    }

    /// `ctx.requestShutdown()` — ask the runner to cancel the run.
    pub fn request_shutdown(&self) {
        self.shutdown.set(true);
    }

    /// Whether [`ExecutionContext::request_shutdown`] was called.
    pub fn shutdown_requested(&self) -> bool {
        self.shutdown.get()
    }

    /// Drain queued `emit` events (runner-internal).
    pub fn take_emitted(&self) -> Vec<Event> {
        std::mem::take(&mut *self.emitted.borrow_mut())
    }

    /// Drain queued `inject` specs (runner-internal).
    pub fn take_injected(&self) -> Vec<Value> {
        std::mem::take(&mut *self.injected.borrow_mut())
    }

    /// `ctx.run_sub(campaign, options)` — run `campaign` nested inside
    /// this run and return the sub-run's event log (ADR-0022, the
    /// `subworkflow` driver's side channel). The sub-run is a fully
    /// isolated engine instance — own bus/log/state/timers — so
    /// nothing it emits reaches the parent namespace; the driver maps
    /// results back explicitly. The invoke blocks for the sub-run's
    /// duration (the runner is a sequential walker — acceptable for
    /// v1). Detached contexts have no channel and fail here.
    pub fn run_sub(
        &self,
        campaign: &Campaign,
        options: &SubRunOptions,
    ) -> Result<Vec<Event>, DriverError> {
        match &self.sub_runner {
            Some(run) => run(campaign, options),
            None => Err(DriverError::new(
                "no sub-run channel — the context is detached from a run",
            )),
        }
    }

    /// `ctx.secrets[key]` — proxy semantics: absent key returns `""`
    /// and, in strict mode, records the miss.
    pub fn secret(&self, key: &str) -> String {
        self.get_secret_or(key, "")
    }

    /// `ctx.getSecret(key)` — absent key records a strict-mode miss
    /// and returns `""`.
    pub fn get_secret(&self, key: &str) -> String {
        self.get_secret_or(key, "")
    }

    /// `ctx.getSecret(key, default)` — a non-empty default suppresses
    /// the strict-mode miss (TS `default_ === ""` check). A miss is
    /// only recorded after every leg of [`ExecutionContext::resolve_secret`]
    /// misses — an ambient-env or store hit is *not* a missing secret.
    pub fn get_secret_or(&self, key: &str, default: &str) -> String {
        if let Some(value) = self.resolve_secret(key) {
            return value;
        }
        if self.strict && default.is_empty() {
            self.missing_secrets.borrow_mut().push(key.to_string());
        }
        default.to_string()
    }

    /// The secret-resolution chain, in precedence order — `None`
    /// when no leg has `key`:
    ///
    /// 1. The run secrets record (`RunOptions.secrets`) — explicit
    ///    caller-provided values override every ambient source.
    /// 2. The ambient process environment (`export NAME=…` — the
    ///    `arca run` channel; empty values count as unset).
    /// 3. [`ExecutionContext::secret_stores`], in order — the
    ///    composition root's keyring and env-file legs. A
    ///    `declared_only` source (the OS keyring) is skipped unless
    ///    the merged `capabilities.secrets` grant matches `key`:
    ///    declaration is the opt-in that keeps a miss from spawning
    ///    `secret-tool` for arbitrary names and from reaching
    ///    credentials a campaign never asked for.
    ///
    /// `capabilities.secrets` gates *resolution surfaces* (the
    /// `${secret.*}` gate and the strict `check_capability` grant
    /// match run before this lookup in template resolution, exactly
    /// as before); the env leg is another source behind the same
    /// gates, not a bypass — `get_secret` itself was never
    /// capability-gated, and ambient env follows the same rule.
    /// Values are never logged; the secrets record stays the only
    /// thing serialized into run state.
    pub fn resolve_secret(&self, key: &str) -> Option<String> {
        if let Some(value) = self.secrets.get(key) {
            return Some(value.clone());
        }
        if let Ok(value) = std::env::var(key)
            && !value.is_empty()
        {
            return Some(value);
        }
        for source in self.secret_stores.iter() {
            if source.declared_only && !self.secret_declared(key) {
                continue;
            }
            if let Some(value) = source.store.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Whether the merged `capabilities.secrets` grant declares
    /// `key` — `true` allows every name, a list is glob alternatives,
    /// `{allow, deny}` is a glob policy ([`peregrin_core::capability_matches`],
    /// the same matching the strict template gate applies). Absent
    /// or `false` declares nothing.
    fn secret_declared(&self, key: &str) -> bool {
        self.capabilities
            .get("secrets")
            .is_some_and(|grant| peregrin_core::capability_matches(key, grant))
    }
}

/// One fallback store leg of [`ExecutionContext::resolve_secret`] —
/// a `peregrin_secret::Secrets` backend plus whether lookups into it
/// require the name to be capability-declared.
#[derive(Clone)]
pub struct SecretSource {
    /// The backend consulted on a record+env miss.
    pub store: Rc<dyn peregrin_secret::Secrets>,
    /// `true` → consulted only when the merged
    /// `capabilities.secrets` grant matches the name — the
    /// OS-keyring leg's opt-in (a per-lookup `secret-tool` spawn and
    /// a reach into OS credentials should never happen for an
    /// undeclared name). `false` → always consulted (the env file,
    /// historically the always-available leg).
    pub declared_only: bool,
}

/// How a driver resolves another driver by name to run it as a
/// sub-effect inside its own `invoke` — `git`'s `params.inner`,
/// `council`'s voters. Bound at the composition level (the runner's
/// `Registry` — `GitDriver::new(|name| registry.resolve(name).ok())`);
/// a [`HashMap`] or closure serves tests. Exact-name match only —
/// `Registry`-style `:`-prefix resolution is the closure's job.
pub trait DriverLookup: Send + Sync {
    /// The driver registered under `name`, if any.
    fn driver(&self, name: &str) -> Option<Arc<dyn Driver>>;
}

impl<F> DriverLookup for F
where
    F: Fn(&str) -> Option<Arc<dyn Driver>> + Send + Sync,
{
    fn driver(&self, name: &str) -> Option<Arc<dyn Driver>> {
        self(name)
    }
}

impl<S> DriverLookup for HashMap<String, Arc<dyn Driver>, S>
where
    S: std::hash::BuildHasher + Send + Sync,
{
    fn driver(&self, name: &str) -> Option<Arc<dyn Driver>> {
        self.get(name).cloned()
    }
}

/// The driver port — invoked by the engine when a node fires
/// (TS `Driver.invoke`). Returns the events the action produced;
/// the engine maps them onto the graph.
pub trait Driver: Send + Sync {
    /// Registry name matched against `node.driver`.
    fn name(&self) -> &str;
    /// Default input contract — used when the node doesn't declare
    /// `input:` itself (TS `driver.input`).
    fn input(&self) -> Option<EventSchema> {
        None
    }
    /// Output contracts keyed by emitted event type (TS `driver.output`).
    fn output(&self) -> &[EventSchema] {
        &[]
    }
    /// Ingress drivers (`github-webhook`, `sturn-poll`) mark
    /// themselves — the runner uses this to keep a campaign that
    /// declares one alive at quiet instead of terminating. Worker/
    /// egress drivers leave the default.
    fn is_ingress(&self) -> bool {
        false
    }
    /// Opt-in to being invoked under `options.dry_run` — the default
    /// short-circuit emits `emits[0]` with `{dry_run: true}` without
    /// ever calling [`Driver::invoke`]. Temporal drivers (`delay`,
    /// `schedule` — ADR-0018) opt in so `--dry-run` reports the
    /// computed schedule (the `until`/`next_fire` fields) rather than
    /// a bare marker; an invoke under dry-run must stay
    /// side-effect-free — no sleeping, no `ctx.schedule_at`.
    fn invoke_in_dry_run(&self) -> bool {
        false
    }
    /// Run the node; emitted events feed the graph.
    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError>;
}

/// `driver: noop` — the trivial driver from `ac-driver-noop`; emits a
/// bare `node.done` event. Useful for tests and pipeline scaffolding.
pub struct NoopDriver;

impl Driver for NoopDriver {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn invoke(
        &self,
        node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        Ok(vec![Event::new(
            "node.done",
            Value::Object(Map::new()),
            &node.id,
        )])
    }
}

/// `driver: runner` — the `ac-driver-runner` port: injects nodes read
/// from the trigger event's `payload.nodes` (or `payload.workflow.nodes`)
/// into the bus mid-run, then emits `{injected: N}`.
pub struct RunnerDriver;

/// The node-spec array a trigger carries. Resolved fields, in order:
/// `payload.nodes` or `payload.workflow.nodes` as a JSON array — then
/// the same fields *as JSON strings*, then `payload.stdout` (a shell
/// planner's only channel — its event payload is `{stdout, rc, cmd}`,
/// so `jq -n '{nodes: [...]}'` output arrives as text). A parsed
/// string may be a bare array or an object wrapping `nodes` /
/// `workflow.nodes`. Anything else yields an empty list — injection
/// is additive, never destructive.
fn candidate_node_specs(payload: &Value) -> Vec<Value> {
    let nodes_at = |value: &Value| {
        value
            .get("nodes")
            .and_then(Value::as_array)
            .or_else(|| {
                value
                    .get("workflow")
                    .and_then(|w| w.get("nodes"))
                    .and_then(Value::as_array)
            })
            .cloned()
    };
    if let Some(items) = nodes_at(payload) {
        return items.iter().filter(|v| v.is_object()).cloned().collect();
    }
    for text in [
        payload.get("nodes").and_then(Value::as_str),
        payload.get("stdout").and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    {
        if let Ok(parsed) = serde_json::from_str::<Value>(text) {
            let items = parsed.as_array().cloned().or_else(|| nodes_at(&parsed));
            if let Some(items) = items {
                return items.iter().filter(|v| v.is_object()).cloned().collect();
            }
        }
    }
    Vec::new()
}

impl Driver for RunnerDriver {
    fn name(&self) -> &'static str {
        "runner"
    }

    fn invoke(
        &self,
        node: &Node,
        triggers: &[Event],
        ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        let Some(trigger) = triggers.first() else {
            return Ok(Vec::new());
        };
        let emit_type = node
            .emits
            .first()
            .cloned()
            .unwrap_or_else(|| format!("{}.done", node.id));
        if ctx.dry_run {
            return Ok(vec![Event::new(
                emit_type,
                serde_json::json!({ "dry_run": true, "injected": 0 }),
                &node.id,
            )]);
        }
        let new_nodes = candidate_node_specs(&trigger.payload);
        if !new_nodes.is_empty() {
            ctx.inject(&new_nodes);
        }
        Ok(vec![Event::new(
            emit_type,
            serde_json::json!({ "injected": new_nodes.len() }),
            &node.id,
        )])
    }
}
