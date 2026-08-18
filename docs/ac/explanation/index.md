# Explanation

Explanation helps you understand why Agent Campaigns is designed the way it is.

- [Abstraction via event types](abstraction.md) — why events are the interface.
- [Adapter ports](adapter-ports.md) — persistence and observability as swappable ports.
- [Drivers](drivers.md) — what drivers are and how the registry works.
- [Event graph model](event-graph-model.md) — why the engine uses an event graph, not a DAG.
- [Events](events.md) — the role of events as flow and state.
- [Handlers](handlers.md) — the role of Python handlers.
- [Include and snippet resolution](include-resolution.md) — how fragments compose.
- [Loop protection](loop-protection.md) — three layers of safety.
- [Observers](observers.md) — the observability adapter.
- [Plugins](plugins.md) — why YAML files are plugins.
- [Resilience](resilience.md) — the resilience model.
- [Safety](safety.md) — how the engine keeps AI-governed workflows safe.
- [Self-modifying workflows](self-modifying.md) — how the graph changes at runtime.
- [Stores](stores.md) — the persistence adapter.
- [Threat model](threat-model.md) — who trusts whom, and what happens when trust is wrong.
