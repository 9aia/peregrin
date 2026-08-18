# Stores (persistence)

The event log flows through an `EventStore` adapter. The bus appends events;
the runner reads them. Swap implementations without touching the bus or runner.

## Built-in stores

- `JsonStore` (default) — append-only JSON lines file. Crash-recoverable by
  default. Default path: `.ac/<workflow-name>.jsonl`.
- `MemoryStore` — in-memory, fast, no persistence. Opt out with `--no-store`.

## Why an adapter port?

The event log IS the workflow state. Different deployments need different
storage: a file for single-process production (default), a database for
distributed workflows, in-memory for tests. The adapter port lets you swap
without touching the core.

The store also provides `seen_types()` — an O(1) lookup for which event
types have been seen. The bus uses this for `mode: all` eligibility checks
instead of scanning the full log.

## Crash recovery by default

The default JsonStore persists every event as it happens. If the process
crashes, the event log is on disk. Use `--replay` to resume from where you
left off. No config needed.

## See also

- [JSON store](../reference/json-store.md)
- [Memory store](../reference/memory-store.md)
- [Write a custom store](../how-to/write-a-custom-store.md)
- [Adapter ports](adapter-ports.md)
