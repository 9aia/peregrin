//! The runner — port of `runner.ts`'s `RunnerLive`: the sequential,
//! queue-driven graph walker.
//!
//! Concurrency model: the TS dispatches each `ready` batch with
//! `Effect.forEach(…, { concurrency: "unbounded" })`, but all
//! cross-node state (`pending`, `ready`, fire counts) is only touched
//! at yield points, so a sequential batch walk is semantically
//! equivalent — deterministic where the TS was interleaved. The one
//! observable divergence is ordering *within* a batch: the TS
//! interleaves `ctx.emit` side events across concurrently-running
//! nodes, then appends each node's returned events; this runner
//! appends side events then node events, per node, in `ready` order.
//!
//! Poll timers: the TS schedules `<id>.tick` re-fires via a forked
//! `Effect.sleep` racing the main loop — a tick only lands if the run
//! is still alive when the interval elapses. The deadline heap here
//! keeps that exact observable behaviour without sleeping: due timers
//! drain at the top of the loop; an emptied loop still exits.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use peregrin_bus::{EventBus, LocalBus, NoopObserver, Observer, capture_wildcards};
use peregrin_core::{Campaign, Event, EventSchema, FunctionDefinition, MaxFires, Node, js_string};
use peregrin_driver::{Driver, DriverError, ExecutionContext};
use peregrin_store::{DeadLetterQueue, MemoryStore, NoopDlq, StateStore};
use peregrin_template::{
    Context, Effects, EventScope, NodeScope, Options, ParamsScope, Scope, StateScope,
    SystemEffects, TemplateError, TemplateFunction, resolve_params, resolve_template_opts,
};

use crate::ingress::Ingress;
use crate::registry::Registry;
use crate::safety::{
    CapabilityChecker, ResourceGate, build_validator, validate_object, validate_record,
};

/// `runWorkflow` options (TS `RunOptions`).
#[derive(Default)]
pub struct RunOptions {
    /// `dryRun` — drivers are skipped; each node emits
    /// `{dry_run: true}` on its first `emits` type.
    pub dry_run: bool,
    /// The run's secrets — read via `ctx.get_secret`/`ctx.secret`.
    pub secrets: BTreeMap<String, String>,
    /// `replayEvents` — a prior log to seed the run with (the
    /// `seed`+`checkReady` path instead of publishing `start`).
    pub replay_events: Vec<Event>,
}

/// Per-node circuit-breaker state (TS `circuitStates` map values).
#[derive(Clone, Copy, Default)]
struct CircuitState {
    failures: u64,
    open: bool,
    opened_at: i64,
}

/// A scheduled `<id>.tick` re-fire (poll nodes, `interval > 0`).
struct ScheduledTick {
    deadline: Instant,
    seq: u64,
    event: Event,
}

impl PartialEq for ScheduledTick {
    fn eq(&self, other: &Self) -> bool {
        (self.deadline, self.seq) == (other.deadline, other.seq)
    }
}
impl Eq for ScheduledTick {}
impl PartialOrd for ScheduledTick {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ScheduledTick {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Min-heap on (deadline, seq).
        (other.deadline, other.seq).cmp(&(self.deadline, self.seq))
    }
}

/// Channels shared by every `ExecutionContext` in a run — the fields
/// the TS hung off the single shared context (`emit`, `inject`,
/// `requestShutdown`, the secrets record). `missingSecrets` is *not*
/// here: the TS creates a fresh array per `makeContext` call, so it
/// lives on the ctx itself.
#[derive(Clone)]
struct Shared {
    secrets: Rc<BTreeMap<String, String>>,
    emitted: Rc<RefCell<Vec<Event>>>,
    injected: Rc<RefCell<Vec<Value>>>,
    shutdown: Rc<Cell<bool>>,
}

impl Shared {
    fn new(secrets: BTreeMap<String, String>) -> Self {
        Self {
            secrets: Rc::new(secrets),
            emitted: Rc::new(RefCell::new(Vec::new())),
            injected: Rc::new(RefCell::new(Vec::new())),
            shutdown: Rc::new(Cell::new(false)),
        }
    }

    fn take_emitted(&self) -> Vec<Event> {
        std::mem::take(&mut *self.emitted.borrow_mut())
    }

    fn take_injected(&self) -> Vec<Value> {
        std::mem::take(&mut *self.injected.borrow_mut())
    }
}

/// The invariant pieces `makeContext` needs — bundled so
/// `run_with_retry` can rebuild a ctx per attempt without a closure
/// borrowing `self`.
struct CtxSpec<'a> {
    node: &'a Node,
    fires: u64,
    campaign: &'a Campaign,
    dry_run: bool,
    shared: &'a Shared,
    log: Rc<Vec<Event>>,
    state: Rc<RefCell<Box<dyn StateStore>>>,
    /// The fire's resolved params — `${params.*}` binds it for
    /// driver-side re-resolves; empty before resolution.
    params: Rc<Map<String, Value>>,
    traceparent: String,
}

/// `ScopedStateStore` — the per-state-key view of the shared store
/// (TS `ScopedStateStore`): `state.key` resolves to a record that is
/// this node's state for the duration of the fire.
struct ScopedStateStore {
    data: Map<String, Value>,
}

impl ScopedStateStore {
    fn new(initial: Map<String, Value>) -> Self {
        Self { data: initial }
    }
}

impl StateStore for ScopedStateStore {
    fn get(&self, key: &str) -> Option<Value> {
        self.data.get(key).cloned()
    }

    fn set(&mut self, key: &str, value: Value) {
        self.data.insert(key.to_string(), value);
    }

    fn to_record(&self) -> Map<String, Value> {
        self.data.clone()
    }

    fn clear(&mut self) {
        self.data.clear();
    }
}

/// The slot a driver-backed custom function pulls its live
/// `ExecutionContext` from — set to the resolve-time ctx before each
/// template call, mirroring the TS passing `ctx` into `fn.call`.
type ExecSlot = Rc<RefCell<Option<ExecutionContext>>>;

/// `RunnerLive` — owns the bus, registry, observer, resource gate,
/// DLQ, state store, and the ambient template [`Effects`].
pub struct Runner {
    bus: Box<dyn EventBus>,
    registry: Registry,
    observer: Arc<dyn Observer>,
    gate: ResourceGate,
    dlq: Box<dyn DeadLetterQueue>,
    state: Rc<RefCell<Box<dyn StateStore>>>,
    effects: Rc<dyn Effects>,
    /// The ingress channel's receiver half — external events drained
    /// into `pending` once per loop iteration (`None` = the run only
    /// sees internally-produced events, the historical behaviour).
    ingress: Option<Ingress>,

    idempotency_counter: u64,
    retry_counts: HashMap<String, u64>,
    circuit_states: HashMap<String, CircuitState>,
    timers: std::collections::BinaryHeap<ScheduledTick>,
    timer_seq: u64,
}

impl Runner {
    /// A runner over `registry` with all-default ports: memory bus and
    /// state store, noop observer, noop DLQ, system effects.
    #[must_use]
    pub fn new(registry: Registry) -> Self {
        Self {
            bus: Box::new(LocalBus::memory()),
            registry,
            observer: Arc::new(NoopObserver),
            gate: ResourceGate::default(),
            dlq: Box::new(NoopDlq),
            state: Rc::new(RefCell::new(Box::new(MemoryStore::default()))),
            effects: Rc::new(SystemEffects),
            ingress: None,
            idempotency_counter: 0,
            retry_counts: HashMap::new(),
            circuit_states: HashMap::new(),
            timers: std::collections::BinaryHeap::new(),
            timer_seq: 0,
        }
    }

    /// Swap the bus (custom store or observer wiring).
    #[must_use]
    pub fn with_bus(mut self, bus: Box<dyn EventBus>) -> Self {
        self.bus = bus;
        self
    }

    /// Swap the observer.
    #[must_use]
    pub fn with_observer(mut self, observer: Arc<dyn Observer>) -> Self {
        self.observer = observer;
        self
    }

    /// Swap the dead-letter queue (ADR-0002).
    #[must_use]
    pub fn with_dlq(mut self, dlq: Box<dyn DeadLetterQueue>) -> Self {
        self.dlq = dlq;
        self
    }

    /// Swap the run state store.
    #[must_use]
    pub fn with_state_store(mut self, state: Box<dyn StateStore>) -> Self {
        self.state = Rc::new(RefCell::new(state));
        self
    }

    /// Swap the ambient effects backing template builtins.
    #[must_use]
    pub fn with_effects(mut self, effects: Rc<dyn Effects>) -> Self {
        self.effects = effects;
        self
    }

    /// Attach an ingress channel — external producers push `Event`s
    /// through the matching [`crate::Injector`] and the run loop
    /// drains them into `pending` once per iteration, serializing
    /// them after the events that triggered the current step.
    #[must_use]
    pub fn with_ingress(mut self, ingress: Ingress) -> Self {
        self.ingress = Some(ingress);
        self
    }

    /// Read the bus's event store (e.g. to inspect the log after a run).
    #[must_use]
    pub fn log(&self) -> Vec<Event> {
        self.bus.log()
    }

    /// `runner.run(workflow, options)` — the event loop: publish
    /// `start` (or seed replay events), then alternate firing the
    /// `ready` batch and publishing the `pending` queue until both
    /// drain or the run cancels. Returns the event log.
    ///
    /// # Errors
    /// `DriverError` on strict-mode preflight violations, unresolvable
    /// drivers, and `state.key`/`skip_if` template failures — the TS
    /// `Effect` failure channel.
    pub fn run(
        &mut self,
        campaign: &Campaign,
        options: &RunOptions,
    ) -> Result<Vec<Event>, DriverError> {
        if campaign.strict {
            for event_schema in &campaign.events {
                if event_schema.compiled_schema().is_none() {
                    return Err(DriverError::new(format!(
                        "strict mode requires a payload schema for event '{}'",
                        event_schema.kind
                    )));
                }
            }
            for node in &campaign.nodes {
                if node.on.is_empty() {
                    return Err(DriverError::new(format!(
                        "strict mode: node '{}' must declare 'on'",
                        node.id
                    )));
                }
                if node.emits.is_empty() {
                    return Err(DriverError::new(format!(
                        "strict mode: node '{}' must declare 'emits'",
                        node.id
                    )));
                }
            }
        }

        let validator = build_validator(&campaign.events);
        self.bus.reset(&campaign.nodes, &campaign.config);
        self.reset_state();
        let mut pending: Vec<Event> = Vec::new();
        let mut ready: Vec<(Node, Vec<Event>)> = Vec::new();
        let mut failed = false;
        let mut cancelled = false;
        let shared = Shared::new(options.secrets.clone());

        self.observer.on_workflow_start(campaign);
        let start = if options.replay_events.is_empty() {
            self.bus.publish(&Event::start())
        } else {
            self.bus.seed(&options.replay_events);
            self.bus.check_ready()
        };
        ready.extend(start.ready);
        pending.extend(start.side_events);

        // Draining happens *before* the exit check so events (and due
        // ticks) that arrive while the last step executes still land —
        // a check-first loop would drop anything injected during the
        // final publish batch.
        loop {
            self.drain_timers(&mut pending);
            self.drain_ingress(&mut pending);
            if (pending.is_empty() && ready.is_empty()) || cancelled {
                break;
            }
            if !ready.is_empty() {
                let batch = std::mem::take(&mut ready);
                for (node, triggers) in batch {
                    let emitted =
                        self.execute_node(&node, &triggers, campaign, options, &shared)?;
                    pending.extend(shared.take_emitted());
                    let injected = shared.take_injected();
                    if !injected.is_empty() {
                        self.bus.inject(&injected);
                    }
                    let has_failure = emitted.iter().any(|e| {
                        matches!(
                            e.kind.as_str(),
                            "node.failed" | "circuit.open" | "workflow.failed"
                        )
                    });
                    pending.extend(emitted);
                    if has_failure {
                        failed = true;
                    }
                    if shared.shutdown.get() {
                        cancelled = true;
                    }
                }
            }
            if cancelled || pending.is_empty() {
                continue;
            }
            let batch = std::mem::take(&mut pending);
            for event in batch {
                if event.kind == "workflow.cancel" {
                    cancelled = true;
                    break;
                }
                if campaign.stop_on.contains(&event.kind) {
                    cancelled = true;
                    break;
                }
                if let Some(validator) = &validator {
                    let errors = validator.validate(&event);
                    if !errors.is_empty() {
                        pending.push(Event::event_invalid(
                            &event.kind,
                            &event.source,
                            &errors,
                            event.payload.clone(),
                        ));
                        if campaign.strict {
                            pending.push(Event::node_failed(
                                &event.source,
                                &format!("payload validation failed: {}", errors.join("; ")),
                            ));
                        }
                        continue;
                    }
                }
                let result = self.bus.publish(&event);
                pending.extend(result.side_events);
                ready.extend(result.ready);
            }
        }

        if cancelled {
            self.bus.publish(&Event::new(
                "workflow.cancelled",
                serde_json::json!({ "reason": "cancelled" }),
                "__runner__",
            ));
        } else if failed {
            self.bus
                .publish(&Event::workflow_failed("", "node failure"));
        } else {
            self.bus.publish(&Event::done());
        }
        let log = self.bus.log();
        self.observer
            .on_workflow_end(log.len(), cancelled, false, failed);
        Ok(log)
    }

    /// Move externally-injected events into the pending queue —
    /// drained once per loop iteration so they serialize after the
    /// events that triggered the current step.
    fn drain_ingress(&self, pending: &mut Vec<Event>) {
        if let Some(ingress) = &self.ingress {
            ingress.drain_into(pending);
        }
    }

    /// Move due scheduled ticks into the pending queue.
    fn drain_timers(&mut self, pending: &mut Vec<Event>) {
        while self
            .timers
            .peek()
            .is_some_and(|tick| tick.deadline <= Instant::now())
        {
            if let Some(tick) = self.timers.pop() {
                pending.push(tick.event);
            }
        }
    }

    fn reset_state(&mut self) {
        self.idempotency_counter = 0;
        self.retry_counts.clear();
        self.circuit_states.clear();
        self.timers.clear();
        self.timer_seq = 0;
    }

    /// `resourceWeight` — `resources.weight`, min 1.
    fn resource_weight(node: &Node) -> u64 {
        let n = node
            .resources
            .get("weight")
            .map_or(f64::NAN, js_number_coerce);
        if n.is_nan() || n < 1.0 {
            return 1;
        }
        n as u64
    }

    /// Build a template [`Context`] from an execution ctx — the
    /// missing-secrets ledger is shared (`Rc`) both ways.
    fn template_ctx<'a>(&'a self, exec: &ExecutionContext) -> Context<'a> {
        Context {
            strict: exec.strict,
            missing_secrets: exec.missing_secrets.clone(),
            capabilities: (*exec.capabilities).clone(),
            effects: &*self.effects,
        }
    }

    /// `makeContext` — assemble the `ExecutionContext` a node/driver
    /// sees. `capabilities` is the shallow
    /// `{...workflow, ...node}` merge.
    fn make_context(spec: &CtxSpec<'_>, idempotency_key: &str) -> ExecutionContext {
        let mut capabilities = spec.campaign.capabilities.clone();
        for (key, value) in &spec.node.capabilities {
            capabilities.insert(key.clone(), value.clone());
        }
        ExecutionContext {
            config: Rc::new(spec.campaign.config.clone()),
            params: spec.params.clone(),
            secrets: spec.shared.secrets.clone(),
            log: spec.log.clone(),
            emitted: spec.shared.emitted.clone(),
            injected: spec.shared.injected.clone(),
            shutdown: spec.shared.shutdown.clone(),
            state: spec.state.clone(),
            dry_run: spec.dry_run,
            idempotency_key: idempotency_key.to_string(),
            traceparent: spec.traceparent.clone(),
            fires: spec.fires,
            strict: spec.campaign.strict,
            // `missingSecrets: []` — a fresh list per ctx, like the TS.
            missing_secrets: Rc::new(RefCell::new(Vec::new())),
            capabilities: Rc::new(capabilities),
            resources: Rc::new(spec.node.resources.clone()),
            workflow_capabilities: Rc::new(spec.campaign.capabilities.clone()),
            python_imports: Rc::new(spec.campaign.python_imports.clone()),
        }
    }

    /// `buildCustomFunctions` — the workflow's `functions:` as
    /// driver-backed [`TemplateFunction`]s.
    fn build_custom_functions(
        &self,
        campaign: &Campaign,
        slot: &ExecSlot,
    ) -> Vec<TemplateFunction> {
        let mut out = Vec::new();
        for def in &campaign.functions {
            if def.name.is_empty() || def.driver.is_empty() {
                continue;
            }
            out.push(make_custom_function(
                def,
                self.registry.clone(),
                slot.clone(),
            ));
        }
        out
    }

    /// `executeNode` — resolve params, validate input, run the driver
    /// through `run_with_retry`, map results to node events.
    fn execute_node(
        &mut self,
        node: &Node,
        triggers: &[Event],
        campaign: &Campaign,
        options: &RunOptions,
        shared: &Shared,
    ) -> Result<Vec<Event>, DriverError> {
        let mut span = self.observer.on_node_start(node, triggers);
        let log: Rc<Vec<Event>> = Rc::new(self.bus.log());
        let fires = self.bus.fire_count(&node.id);

        let reserved = self.gate.acquire(Self::resource_weight(node));
        let result = self.execute_node_gated(
            node, triggers, campaign, options, shared, log, fires, &mut *span,
        );
        self.gate.release(reserved);
        result
    }

    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    fn execute_node_gated(
        &mut self,
        node: &Node,
        triggers: &[Event],
        campaign: &Campaign,
        options: &RunOptions,
        shared: &Shared,
        log: Rc<Vec<Event>>,
        fires: u64,
        span: &mut dyn peregrin_bus::Span,
    ) -> Result<Vec<Event>, DriverError> {
        let slot: ExecSlot = Rc::new(RefCell::new(None));
        let custom_functions = self.build_custom_functions(campaign, &slot);

        // --- resolve node.params against the pre ctx ----------------
        let pre_spec = CtxSpec {
            node,
            fires,
            campaign,
            dry_run: options.dry_run,
            shared,
            log: log.clone(),
            state: self.state.clone(),
            params: Rc::new(Map::new()),
            traceparent: String::new(),
        };
        let pre_ctx = Self::make_context(&pre_spec, &format!("{}:pre", node.id));
        *slot.borrow_mut() = Some(pre_ctx.clone());
        let tctx = self.template_ctx(&pre_ctx);
        let state_view = |key: &str| self.state.borrow().get(key);
        let resolved = resolve_params(
            &node.params,
            &tctx,
            &Options {
                node: Some(node),
                functions: &custom_functions,
                scope: scope_for(
                    &pre_ctx,
                    node,
                    triggers,
                    ParamsScope::declared(),
                    &state_view,
                ),
            },
        );
        let resolved_params = match resolved {
            Err(error) => {
                let fail = Event::node_failed(&node.id, &error.reason);
                self.observer.on_node_end(
                    node,
                    std::slice::from_ref(&fail),
                    Some(&error.reason),
                    span,
                );
                return Ok(vec![fail]);
            }
            Ok(value) => Rc::new(value.as_object().cloned().unwrap_or_default()),
        };

        if node.driver.is_empty() {
            self.observer.on_node_end(node, &[], None, span);
            return Ok(Vec::new());
        }
        let driver = self.registry.resolve(&node.driver)?;

        // --- input schema (node.input ?? driver.input) --------------
        let input_schema = node.input.clone().or_else(|| driver.input());
        if let Some(schema) = &input_schema {
            let input_errors = validate_object(schema, &resolved_params);
            if !input_errors.is_empty() {
                let invalid = Event::event_invalid(
                    "node.input",
                    &node.id,
                    &input_errors,
                    Value::Object((*resolved_params).clone()),
                );
                if campaign.strict {
                    let fail = Event::node_failed(
                        &node.id,
                        &format!("input validation failed: {}", input_errors.join("; ")),
                    );
                    self.observer
                        .on_node_end(node, &[invalid.clone(), fail.clone()], None, span);
                    return Ok(vec![invalid, fail]);
                }
                self.observer
                    .on_node_end(node, std::slice::from_ref(&invalid), None, span);
                return Ok(vec![invalid]);
            }
        }

        // --- node state (state.key / state.skip_if) ------------------
        let mut state_key = String::new();
        let mut node_state = self.state.clone();
        if let Some(state_key_template) = node.state.get("key").and_then(Value::as_str) {
            if !state_key_template.is_empty() {
                *slot.borrow_mut() = Some(pre_ctx.clone());
                let tctx = self.template_ctx(&pre_ctx);
                state_key = resolve_template_opts(
                    state_key_template,
                    &tctx,
                    &Options {
                        node: Some(node),
                        functions: &custom_functions,
                        scope: scope_for(
                            &pre_ctx,
                            node,
                            triggers,
                            ParamsScope::Resolved(&resolved_params),
                            &state_view,
                        ),
                    },
                )
                .map_err(|error| DriverError::new(error.reason))?;
                let initial = match self.state.borrow().get(&state_key) {
                    Some(Value::Object(record)) => record,
                    _ => Map::new(),
                };
                node_state = Rc::new(RefCell::new(Box::new(ScopedStateStore::new(initial))));
                if let Some(skip_if) = node.state.get("skip_if").and_then(Value::as_str) {
                    if !skip_if.is_empty() {
                        let skip_spec = CtxSpec {
                            node,
                            fires,
                            campaign,
                            dry_run: options.dry_run,
                            shared,
                            log: log.clone(),
                            state: node_state.clone(),
                            params: resolved_params.clone(),
                            traceparent: String::new(),
                        };
                        let skip_ctx = Self::make_context(&skip_spec, &format!("{}:skip", node.id));
                        *slot.borrow_mut() = Some(skip_ctx.clone());
                        let tctx = self.template_ctx(&skip_ctx);
                        let scoped_view = |key: &str| node_state.borrow().get(key);
                        let rendered = resolve_template_opts(
                            skip_if,
                            &tctx,
                            &Options {
                                node: Some(node),
                                functions: &custom_functions,
                                scope: scope_for(
                                    &skip_ctx,
                                    node,
                                    triggers,
                                    ParamsScope::Resolved(&resolved_params),
                                    &scoped_view,
                                ),
                            },
                        )
                        .map_err(|error| DriverError::new(error.reason))?;
                        if ["true", "yes", "1", "ok"]
                            .contains(&rendered.trim().to_lowercase().as_str())
                        {
                            let skipped = Event::new(
                                "node.skipped",
                                serde_json::json!({ "node": node.id, "state_key": state_key }),
                                &node.id,
                            );
                            self.observer.on_node_end(
                                node,
                                std::slice::from_ref(&skipped),
                                None,
                                span,
                            );
                            return Ok(vec![skipped]);
                        }
                    }
                }
            }
        }

        // --- capability check ---------------------------------------
        let cap_checker = CapabilityChecker::new(&campaign.capabilities, campaign.strict);
        let effective = cap_checker.merge(&node.capabilities);
        let check = CapabilityChecker::new(&effective, campaign.strict);
        let violations = check.check(&node.driver, &resolved_params);
        if !violations.is_empty() {
            let mut events = vec![Event::capability_violation(&node.id, &violations)];
            if campaign.strict {
                events.push(Event::node_failed(
                    &node.id,
                    &format!("capability violations: {}", violations.join("; ")),
                ));
            }
            self.observer.on_node_end(node, &events, None, span);
            return Ok(events);
        }

        // --- invoke --------------------------------------------------
        let spec = CtxSpec {
            node,
            fires,
            campaign,
            dry_run: options.dry_run,
            shared,
            log: log.clone(),
            state: node_state.clone(),
            params: resolved_params.clone(),
            traceparent: span.traceparent(),
        };
        let driver_events = if node.fan_in.is_some() {
            Vec::new()
        } else {
            self.run_with_retry(&spec, triggers, &driver)
        };

        // --- poll tick scheduling ------------------------------------
        if let Some(poll) = &node.poll {
            let tick = format!("{}.tick", node.id);
            let is_tick = triggers.iter().any(|event| event.kind == tick);
            let interval = node
                .params
                .get("interval")
                .and_then(Value::as_f64)
                .or_else(|| poll.get("interval").and_then(Value::as_f64))
                .unwrap_or(0.0);
            if is_tick && interval > 0.0 {
                self.timer_seq += 1;
                self.timers.push(ScheduledTick {
                    deadline: Instant::now()
                        + Duration::try_from_secs_f64(interval).unwrap_or_default(),
                    seq: self.timer_seq,
                    event: Event::new(tick, serde_json::json!({ "poll": true }), &node.id),
                });
            }
        }

        let all = build_node_events(node, triggers, driver_events.clone());

        // --- strict `emits` declaration check -------------------------
        if campaign.strict {
            for event in &all {
                if matches!(
                    event.kind.as_str(),
                    "node.failed" | "circuit.open" | "node.skipped"
                ) {
                    continue;
                }
                if !node.emits.contains(&event.kind) {
                    let fail = Event::node_failed(
                        &node.id,
                        &format!("emitted '{}' not declared in 'emits'", event.kind),
                    );
                    self.observer
                        .on_node_end(node, std::slice::from_ref(&fail), None, span);
                    return Ok(vec![fail]);
                }
            }
        }

        if !state_key.is_empty() && !driver_events.iter().any(|e| e.kind == "node.failed") {
            self.state
                .borrow_mut()
                .set(&state_key, Value::Object(node_state.borrow().to_record()));
        }

        self.observer.on_node_end(node, &all, None, span);
        Ok(all)
    }

    /// `runWithRetry` — circuit-breaker gate, then `max_attempts`
    /// attempts with backoff; on exhaust, `node.failed` + DLQ enqueue.
    fn run_with_retry(
        &mut self,
        spec: &CtxSpec<'_>,
        triggers: &[Event],
        driver: &Arc<dyn Driver>,
    ) -> Vec<Event> {
        let node = spec.node;
        let mut now = now_millis();
        if self.is_circuit_open(node, now) {
            return vec![Event::node_failed(&node.id, "circuit open")];
        }

        let retry_cfg = &node.retry;
        let max_attempts = retry_cfg.get("max_attempts").map_or(1.0, js_number_coerce);
        let max_attempts = if max_attempts.is_nan() {
            // `Math.max(1, NaN)` is NaN — the TS loop never runs and
            // the node fails with "unknown error".
            0_u64
        } else {
            max_attempts.max(1.0) as u64
        };
        let mut last_error: Option<DriverError> = None;

        let mut attempt = 1_u64;
        while attempt <= max_attempts {
            self.idempotency_counter += 1;
            let idempotency_key = format!("{}:{}", node.id, self.idempotency_counter);
            let ctx = Self::make_context(spec, &idempotency_key);
            let result = Self::run_driver(node, driver, triggers, &ctx, spec.dry_run);
            match result {
                Ok(events) => {
                    self.record_success(node);
                    self.retry_counts.remove(&node.id);
                    return Self::validate_output(node, driver, &events, spec.campaign);
                }
                Err(error) => {
                    last_error = Some(error);
                    now = now_millis();
                    if self.record_failure(node, now) {
                        let on_trip = node
                            .circuit_breaker
                            .get("on_trip")
                            .and_then(Value::as_str)
                            .unwrap_or("circuit.open");
                        return vec![
                            Event::new(
                                on_trip,
                                serde_json::json!({
                                    "node": node.id,
                                    "failures": self
                                        .circuit_states
                                        .get(&node.id)
                                        .map_or(0, |cb| cb.failures),
                                }),
                                &node.id,
                            ),
                            Event::node_failed(
                                &node.id,
                                &format!("circuit breaker tripped for '{}'", node.id),
                            ),
                        ];
                    }
                    self.retry_counts.insert(node.id.clone(), attempt);

                    if attempt < max_attempts {
                        let base_delay = retry_cfg.get("delay").map_or(1.0, js_number_coerce);
                        let backoff = retry_cfg
                            .get("backoff")
                            .and_then(Value::as_str)
                            .unwrap_or("fixed");
                        let delay = match backoff {
                            "exponential" => base_delay * 2_f64.powi(attempt as i32 - 1),
                            "linear" => base_delay * attempt as f64,
                            _ => base_delay,
                        };
                        if delay > 0.0 {
                            std::thread::sleep(
                                Duration::try_from_secs_f64(delay).unwrap_or_default(),
                            );
                        }
                    }
                }
            }
            attempt += 1;
        }

        let fail = Event::node_failed(
            &node.id,
            &last_error.map_or_else(|| "unknown error".to_string(), |e| e.reason),
        );
        self.dlq.enqueue(&fail, "retry exhausted");
        vec![fail]
    }

    /// `runDriver` — dry-run short-circuit, else `driver.invoke`.
    fn run_driver(
        node: &Node,
        driver: &Arc<dyn Driver>,
        triggers: &[Event],
        ctx: &ExecutionContext,
        dry_run: bool,
    ) -> Result<Vec<Event>, DriverError> {
        if dry_run {
            let kind = node
                .emits
                .first()
                .cloned()
                .unwrap_or_else(|| format!("{}.done", node.id));
            return Ok(vec![Event::new(
                kind,
                serde_json::json!({ "dry_run": true }),
                &node.id,
            )]);
        }
        driver.invoke(node, triggers, ctx)
    }

    /// `validateOutput` — driver/node output schemas over the emitted
    /// events; `hasInvalid` adds a `node.failed` under strict.
    fn validate_output(
        node: &Node,
        driver: &Arc<dyn Driver>,
        events: &[Event],
        campaign: &Campaign,
    ) -> Vec<Event> {
        let schemas = resolve_output_schemas(driver.output(), &node.output);
        if schemas.is_empty() {
            return events.to_vec();
        }
        let mut out = Vec::new();
        let mut has_invalid = false;
        for event in events {
            let Some(schema) = matching_output_schema(&schemas, &event.kind) else {
                out.push(event.clone());
                continue;
            };
            let errors = validate_event_payload(schema, &event.payload);
            if errors.is_empty() {
                out.push(event.clone());
            } else {
                has_invalid = true;
                out.push(Event::event_invalid(
                    &event.kind,
                    &node.id,
                    &errors,
                    event.payload.clone(),
                ));
            }
        }
        if has_invalid && campaign.strict {
            out.push(Event::node_failed(
                &node.id,
                "driver output validation failed",
            ));
        }
        out
    }

    /// `isCircuitOpen` — open and not yet past `reset_after`.
    fn is_circuit_open(&mut self, node: &Node, now: i64) -> bool {
        let Some(cb) = self.circuit_states.get_mut(&node.id) else {
            return false;
        };
        if !cb.open {
            return false;
        }
        let reset_after = node
            .circuit_breaker
            .get("reset_after")
            .map_or(60.0, js_number_coerce)
            * 1000.0;
        if (now - cb.opened_at) as f64 > reset_after {
            cb.open = false;
            cb.failures = 0;
            return false;
        }
        true
    }

    /// `recordFailure` — count a failure; returns `true` the moment the
    /// breaker trips (failures ≥ `threshold`, default 5). No
    /// `circuit_breaker` block means no breaker.
    fn record_failure(&mut self, node: &Node, now: i64) -> bool {
        if node.circuit_breaker.is_empty() {
            return false;
        }
        let cb = self.circuit_states.entry(node.id.clone()).or_default();
        cb.failures += 1;
        let threshold = node
            .circuit_breaker
            .get("threshold")
            .map_or(5.0, js_number_coerce);
        if !cb.open && cb.failures as f64 >= threshold {
            cb.open = true;
            cb.opened_at = now;
            return true;
        }
        false
    }

    /// `recordSuccess` — a success resets the breaker.
    fn record_success(&mut self, node: &Node) {
        if let Some(cb) = self.circuit_states.get_mut(&node.id) {
            cb.failures = 0;
            cb.open = false;
        }
    }
}

/// `Event.ts`-level `Date.now()` for the circuit-breaker clock.
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// JS `Number(v)` on a JSON value — numbers pass, numeric strings
/// parse, booleans count, everything else is `NaN`.
fn js_number_coerce(value: &Value) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
        Value::Bool(b) => f64::from(u8::from(*b)),
        _ => f64::NAN,
    }
}

/// The [`Scope`] one template resolution at this fire binds — `event`
/// is `triggers[0]` plus the node's `on` patterns (wildcard captures),
/// `config`/`secrets`/`node` come from the ctx; `params` and `state`
/// vary by call site (declared-vs-resolved params, shared-vs-scoped
/// store).
fn scope_for<'a>(
    exec: &'a ExecutionContext,
    node: &'a Node,
    triggers: &'a [Event],
    params: ParamsScope<'a>,
    state: &'a dyn StateScope,
) -> Scope<'a> {
    Scope {
        event: triggers.first().map(|event| EventScope {
            event,
            patterns: &node.on,
        }),
        config: Some(&exec.config),
        params,
        state: Some(state),
        node: Some(NodeScope {
            id: &node.id,
            fires: exec.fires,
        }),
        secrets: Some(&exec.secrets),
    }
}

/// `resolveForward` — substitute wildcard captures into a `forward`
/// event type (`*`-parts take `caps["1"]`, `caps["2"]`, …).
fn resolve_forward(fwd: &str, captures: &HashMap<String, String>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut cap_idx = 1;
    for part in fwd.split('.') {
        if part == "*" || part == "**" {
            out.push(
                captures
                    .get(&cap_idx.to_string())
                    .cloned()
                    .unwrap_or_else(|| "*".to_string()),
            );
            cap_idx += 1;
        } else {
            out.push(part.to_string());
        }
    }
    out.join(".")
}

/// `resolveOutputSchemas` — driver schemas overridden by node schemas,
/// keyed by event type, preserving declaration order (the TS `Map`).
fn resolve_output_schemas(driver: &[EventSchema], node: &[EventSchema]) -> Vec<EventSchema> {
    let mut out: Vec<EventSchema> = Vec::new();
    for schema in driver.iter().chain(node.iter()) {
        match out.iter_mut().find(|s| s.kind == schema.kind) {
            Some(existing) => *existing = schema.clone(),
            None => out.push(schema.clone()),
        }
    }
    out
}

/// `matchingOutputSchema` — exact type first, then wildcard match.
fn matching_output_schema<'a>(schemas: &'a [EventSchema], kind: &str) -> Option<&'a EventSchema> {
    for schema in schemas {
        if schema.kind == kind {
            return Some(schema);
        }
    }
    schemas
        .iter()
        .find(|schema| schema.kind.contains('*') && peregrin_bus::matches(&schema.kind, kind))
}

/// `validateObject(schema, event.payload)` — the driver-output arm of
/// payload validation; non-object payloads report `(root)`.
fn validate_event_payload(schema: &EventSchema, payload: &Value) -> Vec<String> {
    let Some(compiled) = schema.compiled_schema() else {
        return Vec::new();
    };
    let Some(schema_map) = compiled.as_object() else {
        return Vec::new();
    };
    match payload.as_object() {
        Some(map) => validate_record(schema_map, map),
        None => vec!["(root): payload must be an object".to_string()],
    }
}

/// `buildNodeEvents` — fan-in emit / driver events, then `forward`
/// re-emit per trigger, then the `interval <= 0` poll self-tick.
fn build_node_events(node: &Node, triggers: &[Event], driver_events: Vec<Event>) -> Vec<Event> {
    let mut out: Vec<Event> = Vec::new();
    if let Some(fan_in) = &node.fan_in {
        let emit_type = fan_in
            .get("emit")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| node.emits.first().cloned())
            .unwrap_or_else(|| format!("{}.done", node.id));
        out.push(Event::new(
            emit_type,
            serde_json::json!({
                "count": triggers.len(),
                "triggers": triggers
                    .iter()
                    .map(|trigger| serde_json::json!({
                        "type": trigger.kind,
                        "source": trigger.source,
                        "payload": trigger.payload,
                    }))
                    .collect::<Vec<_>>(),
            }),
            &node.id,
        ));
    } else {
        out.extend(driver_events);
    }
    for trigger in triggers {
        for fwd in &node.forward {
            let mut resolved = fwd.clone();
            for on_pattern in &node.on {
                if let Some(caps) = capture_wildcards(on_pattern, &trigger.kind) {
                    resolved = resolve_forward(fwd, &caps);
                    break;
                }
            }
            out.push(Event::new(resolved, trigger.payload.clone(), &node.id));
        }
    }
    if let Some(poll) = &node.poll {
        let tick = format!("{}.tick", node.id);
        let interval = node
            .params
            .get("interval")
            .and_then(Value::as_f64)
            .or_else(|| poll.get("interval").and_then(Value::as_f64))
            .unwrap_or(0.0);
        if interval <= 0.0 && !out.iter().any(|e| e.kind == tick) {
            out.push(Event::new(
                tick,
                serde_json::json!({ "poll": true }),
                &node.id,
            ));
        }
    }
    out
}

/// `applyArgs` — `${N}` placeholders in `params` take `args[N]`
/// (JS `String(arg)`); out-of-range placeholders stay literal.
fn apply_args(value: &Value, args: &[Value]) -> Value {
    match value {
        Value::String(text) => Value::String(apply_args_str(text, args)),
        Value::Array(items) => Value::Array(items.iter().map(|v| apply_args(v, args)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), apply_args(v, args)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn apply_args_str(text: &str, args: &[Value]) -> String {
    let mut out = String::new();
    let mut index = 0;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        if bytes[index] == b'$'
            && index + 1 < bytes.len()
            && bytes[index + 1] == b'{'
            && let Some(rel) = text[index + 2..].find('}')
        {
            let inner = &text[index + 2..index + 2 + rel];
            if !inner.is_empty() && inner.bytes().all(|b| b.is_ascii_digit()) {
                let i: usize = inner.parse().unwrap_or(usize::MAX);
                if i < args.len() {
                    out.push_str(&js_string(&args[i]));
                } else {
                    out.push_str("${");
                    out.push_str(inner);
                    out.push('}');
                }
                index += 2 + rel + 1;
                continue;
            }
        }
        match text[index..].chars().next() {
            Some(ch) => {
                out.push(ch);
                index += ch.len_utf8();
            }
            None => break,
        }
    }
    out
}

/// `getPath` — walk a dotted path through objects; `null` where the
/// TS would read `undefined`.
fn get_path(value: &Value, path: &str) -> Value {
    let mut current = value;
    for part in path.split('.') {
        match current {
            Value::Object(map) => match map.get(part) {
                Some(next) => current = next,
                None => return Value::Null,
            },
            _ => return Value::Null,
        }
    }
    current.clone()
}

/// `makeCustomFunction` — a `functions:` definition becomes a
/// driver-backed template function: resolve the driver, apply
/// `${N}`-args to `params`, invoke with a synthetic `fn:{name}` node,
/// return the first event's payload (or `returns` path).
fn make_custom_function(
    def: &FunctionDefinition,
    registry: Registry,
    slot: ExecSlot,
) -> TemplateFunction {
    let def = def.clone();
    let name = def.name.clone();
    TemplateFunction::new(
        &name,
        def.deterministic,
        def.requires_capability,
        move |args, _ctx, _node, _method| {
            let driver = registry
                .resolve(&def.driver)
                .map_err(|error| TemplateError::new(error.reason))?;
            let resolved_params = match apply_args(&Value::Object(def.params.clone()), args) {
                Value::Object(map) => map,
                _ => Map::new(),
            };
            let node = Node {
                id: format!("fn:{}", def.name),
                driver: def.driver.clone(),
                emits: vec!["fn.done".to_string()],
                params: Value::Object(resolved_params),
                capabilities: def.capabilities.clone(),
                max_fires: MaxFires::Fires(1),
                ..Node::default()
            };
            let ctx = slot.borrow().clone().unwrap_or_default();
            let events = driver
                .invoke(&node, &[], &ctx)
                .map_err(|error| TemplateError::new(error.reason))?;
            let Some(first) = events.first() else {
                return Err(TemplateError::new(format!(
                    "function '{}' produced no output",
                    def.name
                )));
            };
            if first.kind.contains("failed") || first.kind == "node.failed" {
                let reason = first
                    .payload
                    .get("error")
                    .and_then(Value::as_str)
                    .map_or_else(
                        || format!("function '{}' failed: {}", def.name, first.kind),
                        str::to_string,
                    );
                return Err(TemplateError::new(reason));
            }
            Ok(match &def.returns {
                Some(path) => get_path(&first.payload, path),
                None => first.payload.clone(),
            })
        },
    )
}
