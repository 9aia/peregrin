# Adapter ports

The runner's I/O seams are trait objects, not concrete types. The
`Runner` in `peregrin-engine` composes a bus, an event log, a state
store, a dead-letter queue, and an observer — it does not know or
care which implementation backs any of them.

This is the same ports-and-adapters split the rest of the workspace
follows: policy crates are pure, every I/O seam is a port
(`peregrin-{store,bus,remote,config,secret,service,loader,template}`)
with impls behind it (`-json`, `-toml`, `-yaml`, `-keyring`,
`-dotenv`, `-http`, `-systemd`). See
[Architecture](architecture.md#ports-and-adapters).

## The event log — `EventStore`

The append-only event log IS the workflow state. The bus appends
events; the runner reads them back for `mode: all` eligibility and
trigger selection. The port lives in `peregrin-store`:

```rust
pub trait EventStore: Send {
    fn append(&mut self, event: &Event);
    fn all(&self) -> &[Event];
    fn by_type(&self, kind: &str) -> &[Event];
    fn seen_types(&self) -> Vec<String>;
    fn latest(&self) -> Option<&Event>;
    fn clear(&mut self);
}
```

`seen_types` is the fast path: the bus uses it for `mode: all`
eligibility instead of scanning the full log per publish.

Implementations:

- `MemoryStore` (`peregrin-store`) — in-memory; the ephemeral default
  every `Runner::new` gets.
- `JsonStore` (`peregrin-store-json`) — append-only JSONL file;
  durable across process exit. `peregrin campaign run --history PATH`
  swaps the log onto it; `peregrin runs`/`explain`/`log` read that
  same file back.
- Custom — implement `EventStore` and hand it to `LocalBus::new`.

## Run state — `StateStore`

`ctx.state` and the `${state.*}` scope resolve against a `StateStore`
(`get`/`set`/`to_record`/`clear`). Nodes declaring `state.key` get a
scoped view over one record; everything else shares the run store.
The default is in-memory — the durable `JsonStateStore` half of the
port is a later item (ADR-0013).

## Dead letters — `DeadLetterQueue`

Failures that outlive retry land on a `DeadLetterQueue`
(`enqueue`/`all`). `Runner::new` defaults to `NoopDlq` (dropped);
`peregrin-node`'s campaign plane upgrades it to `MemoryDlq` so
dead-lettered events survive for the life of the run (ADR-0002).

## Observability — `Observer`

Lifecycle hooks flow through `peregrin_bus::Observer`:

```rust
pub trait Observer: Send + Sync {
    fn on_workflow_start(&self, campaign: &Campaign) {}
    fn on_workflow_end(&self, event_count: usize, cancelled: bool, cycle: bool, failed: bool) {}
    fn on_node_start(&self, node: &Node, triggers: &[Event]) -> Box<dyn Span>;
    fn on_node_end(&self, node: &Node, events: &[Event],
                   error: Option<&DriverError>, span: Box<dyn Span>);
    fn on_event(&self, event: &Event) {}
    fn on_inject(&self, node_specs: &[Value]) {}
    fn on_retire(&self, node_id: &str) {}
}
```

`on_node_start` returns a `Span` the runner hands back to
`on_node_end` — the observer decides whether it is a real trace span
or a no-op, and its `traceparent` is what `ctx.traceparent` carries
into drivers. `NoopObserver` is the zero-overhead default. See
[Observers](observers.md).

## Why ports?

The event log is the workflow state, and different deployments need
different storage: a file for single-process runs, a database for
distributed workflows, memory for tests. Observability is
deployment-specific in the same way — a local `campaign run` needs no
tracing, a hosted node wants spans. Ports let each deployment pick
its impls without touching `peregrin-engine` — composition happens
once, in `peregrin_node::campaign`.

## See also

- [Stores](stores.md) — the persistence half in detail
- [Observers](observers.md) — the observability half
- [Event envelope](../reference/event-envelope.md) — what the log
  records
