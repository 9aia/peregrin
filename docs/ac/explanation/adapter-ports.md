# Adapter ports

The runner has two adapter ports — swappable implementations for
persistence and observability. The runner does not know or care which
implementation you use.

## EventStore (persistence)

The event log flows through an `EventStore`. The bus appends events; the
runner reads them. Swap implementations without touching the bus or runner.

```
EventStore protocol:
    append(event)        — add an event
    all()                — return all events in order
    by_type(type)        — return events matching a type
    seen_types()         — set of all event types ever appended
    latest()             — most recent event
    clear()              — remove all events
    close()              — release resources
```

### Implementations

- `JsonStore` (default) — Append-only JSON lines file. Crash-recoverable by
  default. Default path: `.ac/<workflow-name>.jsonl`.
- `MemoryStore` — In-memory, fast, no persistence. Opt out with
  `--no-store` or `store=MemoryStore()`.
- Custom — Implement the protocol (SQLite, Postgres, Redis, ...).

### Why an adapter port?

The event log IS the workflow state. Different deployments need different
storage: a file for single-process production (default), a database for
distributed workflows, in-memory for tests. The adapter port lets you swap
without touching the core.

**Crash recovery by default.** The default JsonStore persists every event
as it happens. If the process crashes, the event log is on disk. Use
`--replay` to resume from where you left off. No config needed.

The store also provides `seen_types()` — an O(1) lookup for which event
types have been seen. The bus uses this for `mode: all` eligibility checks
instead of scanning the full log.

### Writing a custom store

```python
from agent_campaign.model import Event

class MyPostgresStore:
    def __init__(self, dsn: str):
        self._conn = connect(dsn)

    def append(self, event: Event) -> None:
        self._conn.execute("INSERT INTO events ...", ...)

    def all(self) -> list[Event]:
        return [Event(...) for row in self._conn.query("SELECT ...")]

    def by_type(self, event_type: str) -> list[Event]:
        ...

    def seen_types(self) -> set[str]:
        return {row[0] for row in self._conn.query("SELECT DISTINCT type ...")}

    def latest(self) -> Event | None:
        ...

    def clear(self) -> None:
        self._conn.execute("TRUNCATE events")

    def close(self) -> None:
        self._conn.close()
```

## Observer (observability)

Lifecycle hooks flow through an `Observer`. The runner calls the observer
at key points: workflow start/end, node start/end, event emission, node
injection, node retirement.

```
Observer protocol:
    on_workflow_start(workflow)
    on_workflow_end(event_count, cancelled, cycle)
    on_node_start(node, triggers) → Span
    on_node_end(node, events, error, span)
    on_event(event)
    on_inject(node_specs)
    on_retire(node_id)
```

### Implementations

| Observer                 | Use                                         |
| ------------------------ | ------------------------------------------- |
| `NoopObserver` (default) | Silent, zero overhead                       |
| `OTelObserver`           | OpenTelemetry spans + metrics               |
| Custom                   | Implement the protocol (Datadog, logs, ...) |

### Why an adapter port?

Observability is deployment-specific. A local dev run does not need tracing.
A production run needs OTel or Datadog. A debug session might just log.
The adapter port lets you swap without touching the runner.

The `Span` returned by `on_node_start` is a context manager closed by
`on_node_end`. This decouples span lifecycle from the runner — the observer
decides whether to create a real trace span or a no-op.

### Writing a custom observer

```python
class MyLogObserver:
    def on_workflow_start(self, workflow):
        print(f"START: {workflow.name}")

    def on_workflow_end(self, event_count, cancelled, cycle):
        print(f"END: {event_count} events, cancelled={cancelled}")

    def on_node_start(self, node, triggers):
        print(f"  NODE: {node.id}")
        return NoopSpan()

    def on_node_end(self, node, events, error, span):
        if error:
            print(f"  ERROR: {node.id} — {error}")
        span.end()

    def on_event(self, event):
        pass

    def on_inject(self, node_specs):
        print(f"  INJECTED: {len(node_specs)} nodes")

    def on_retire(self, node_id):
        print(f"  RETIRED: {node_id}")
```
