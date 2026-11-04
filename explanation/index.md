# Explanation

Why peregrin is designed the way it is — the concepts behind the
contracts in [Reference](../reference/index.md).

## The model

- [Abstraction via event types](abstraction.md) — why events are the
  interface.
- [Event graph model](event-graph-model.md) — why the engine uses an
  event graph, not a DAG.
- [Events](events.md) — the role of events as flow and state.
- [Drivers](drivers.md) — what drivers are and how the unified
  registry works.
- [Self-modifying workflows](self-modifying.md) — how the graph
  changes at runtime.

## The seams

- [Adapter ports](adapter-ports.md) — persistence and observability
  as swappable ports.
- [Observers](observers.md) — the observability port.
- [Stores](stores.md) — the persistence port.
- [Include and snippet resolution](include-resolution.md) — how
  fragments compose.
- [Plugins](plugins.md) — why YAML files are plugins, and how
  bundles distribute them.

## The bounds

- [Resilience](resilience.md) — retry, DLQ, circuit breaker.
- [Loop protection](loop-protection.md) — the mechanisms that bound
  a run.
- [Safety](safety.md) — how the engine keeps AI-governed workflows
  safe.
- [Threat model](threat-model.md) — who trusts whom, and what
  happens when trust is wrong.

## The whole

- [Architecture](architecture.md) — nodes, hub, the sepia boundary,
  planes.
