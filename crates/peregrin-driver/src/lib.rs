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
use std::collections::BTreeMap;
use std::rc::Rc;

use serde_json::{Map, Value};

use peregrin_core::{Event, EventSchema, Node, PythonImport};
use peregrin_store::StateStore;

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
    /// The run's secrets — read via [`ExecutionContext::secret`] /
    /// [`ExecutionContext::get_secret`] so misses are audited
    /// (TS `secrets` proxy).
    pub secrets: Rc<BTreeMap<String, String>>,
    /// Snapshot of the event log at node start (TS `ctx.log`).
    pub log: Rc<Vec<Event>>,
    /// Events pushed via [`ExecutionContext::emit`] — the runner drains
    /// this into the pending queue after each node fire.
    pub emitted: Rc<RefCell<Vec<Event>>>,
    /// Node specs pushed via [`ExecutionContext::inject`] — the runner
    /// feeds them to `bus.inject`.
    pub injected: Rc<RefCell<Vec<Value>>>,
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
            log: Rc::new(Vec::new()),
            emitted: Rc::new(RefCell::new(Vec::new())),
            injected: Rc::new(RefCell::new(Vec::new())),
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
    /// the strict-mode miss (TS `default_ === ""` check).
    pub fn get_secret_or(&self, key: &str, default: &str) -> String {
        if let Some(value) = self.secrets.get(key) {
            return value.clone();
        }
        if self.strict && default.is_empty() {
            self.missing_secrets.borrow_mut().push(key.to_string());
        }
        default.to_string()
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
        let candidate = trigger
            .payload
            .get("nodes")
            .and_then(Value::as_array)
            .or_else(|| {
                trigger
                    .payload
                    .get("workflow")
                    .and_then(|w| w.get("nodes"))
                    .and_then(Value::as_array)
            });
        let new_nodes: Vec<Value> = candidate.map_or_else(Vec::new, |items| {
            items.iter().filter(|v| v.is_object()).cloned().collect()
        });
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
