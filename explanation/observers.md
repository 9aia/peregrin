# Observers (observability)

Lifecycle hooks flow through the `Observer` port (`peregrin-bus`).
The runner calls it at every state transition — workflow start and
end, node start and end, every published event, node injection, node
retirement — and knows nothing about what backs it.

## The port

```rust
pub trait Observer: Send + Sync {
    fn on_workflow_start(&self, campaign: &Campaign) {}
    fn on_workflow_end(&self, event_count: usize, cancelled: bool, cycle: bool, failed: bool) {}
    fn on_node_start(&self, node: &Node, triggers: &[Event]) -> Box<dyn Span>;
    fn on_node_end(&self, node: &Node, events: &[Event], error: Option<&DriverError>, span: Box<dyn Span>);
    fn on_event(&self, event: &Event) {}
    fn on_inject(&self, node_specs: &[Value]) {}
    fn on_retire(&self, node_id: &str) {}
}
```

Every method has a do-nothing default — implement only the hooks you
need. `NoopObserver` is the stock impl: zero overhead, the default
every `Runner::new`/`LocalBus` gets unless you swap it via
`Runner::with_observer`.

The `Span` returned by `on_node_start` is handed back to
`on_node_end` — the observer decides whether it is a real trace span
or a no-op, decoupling span lifecycle from the runner. Its
`traceparent` is what `ctx.traceparent` carries into drivers, so a
driver emitting into an external system can continue the trace.

## Why a port?

Observability is deployment-specific. A local `peregrin campaign
run` needs the printed event log, nothing more. A hosted node wants
spans and metrics. A debug session wants per-event logging. The port
lets each deployment pick without touching `peregrin-engine`.

On the daemon side, `hypha-telemetry` layers `tracing` (fmt +
`RUST_LOG`) with OTLP span export when `HYPHA_OTEL_ENDPOINT` (or
`OTEL_EXPORTER_OTLP_*`) is set — node and hub observability, one
crate. The engine-level `Observer` is the campaign-run half of the
same seam; v1's `OtelObserver` for runs is not ported yet.

## See also

- [Adapter ports](adapter-ports.md) — the other swappable seams
- [Event envelope](../reference/event-envelope.md) — what `on_event`
  sees
- [Environment](../reference/environment.md) — the `HYPHA_OTEL_*`
  knobs
