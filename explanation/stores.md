# Stores (persistence)

The event log flows through an `EventStore` port; run state through
`StateStore`; dead letters through `DeadLetterQueue`. All three live
in `peregrin-store` — the bus appends, the runner reads, and neither
knows which impl is behind the trait.

## Built-in implementations

- `MemoryStore` — in-memory `EventStore` + `StateStore`; the
  ephemeral default. Every `Runner::new` starts here: `peregrin
  campaign run` prints the returned log instead of persisting it
  (the v1 `handl run` semantics).
- `JsonStore` (`peregrin-store-json`) — append-only JSONL
  `EventStore`; durable across process exit. `peregrin campaign run
  --history <path>` swaps the log onto it, and `peregrin runs`,
  `explain`, and `log` read that same file back — the observability
  verbs are just readers of the store format. Reopened on the same
  path, the store resumes publish/first-seen order — that is what
  makes `--replay` runs possible.
- `MemoryDlq` / `NoopDlq` — dead-letter impls; the node plane keeps
  dead letters for the run's life (`MemoryDlq`), the bare default
  drops them.

Not yet ported: the `JsonStateStore` half of the v1 json adapter —
run state stays in-memory (ADR-0013).

## Why ports?

The event log IS the workflow state. Different deployments need
different storage: a file for single-process runs, a database for
distributed workflows, in-memory for tests. The port lets you swap
without touching the engine — composition happens once, in
`peregrin_node::campaign` (`runner_with_log` is the JSONL-backed
variant).

`seen_types` — the store's O(1)-ish "which types have been
published" view — is what `mode: all` eligibility checks against
instead of scanning the full log per publish.

## Crash recovery by default?

Not quite — the split is explicit in v2: the *default* is ephemeral
(`MemoryStore`, the TS `handl run` behavior), and durability is the
`--history` flag. Deployed campaign units run under `campaign serve
--history-dir`, which appends one JSONL per campaign
(`<dir>/<campaign>.jsonl`), so gates persist every event as it
happens. Use `peregrin campaign run --replay <log>` to seed a run
from a prior log.

## See also

- [Adapter ports](adapter-ports.md) — the trait surfaces
- [Queue & supersede](../reference/queue-supersede.md) — how queued
  triggers relate to the log
- [Event envelope](../reference/event-envelope.md) — what a log line
  contains
