# ADR-0013: Persistent State for Agents and Events

## Status

Accepted

## Context

Peregrin already exposes `ctx.state` as a `StateStore` in `ExecutionContext`, and the
documentation describes it as a "persisted key/value store (default
`.ac/<name>.state.json`), restored on `--replay`". In the current engine, however,
`ctx.state` is a `SimpleStateStore` that is created fresh for every node execution
and discarded afterwards. It is not persisted, not restored, and not shared across
node firings or runs.

This gap blocks a whole class of useful workflows: exhaustive, item-by-item tasks
such as reviewing every module in a repository, running every test, or refactoring
every folder. For these workflows an item needs to be:

- cacheable (skip if already completed),
- resumable (continue after a crash or cancellation), and
- observable (humans or downstream tooling can inspect progress).

A concrete example is a review campaign that iterates over `packages/*`. A node
should be able to remember that `packages/ac-core` is `reviewed: true` and skip it
on the next run, rather than re-running the agent for every module every time.

## Decision

Peregrin will implement a persistent `StateStore` as a first-class port, make
`ctx.state` a per-execution view into that store, and add a declarative `state:`
block on nodes so the engine can load, save, and skip based on persisted values.

### 1. A persistent `StateStore` port

`ac-core` will define a `StateStore` port (similar to `EventStore`) with
get/set/has/delete/toRecord operations. `ac-store-json` and `ac-store-memory` will
implement the port. The CLI will default to a JSONL file at `.ac/<name>.state.jsonl`
unless `--no-store` is used, in which case the in-memory store is used.

### 2. `state:` on nodes

A node may declare a `state:` block:

```yaml
nodes:
  - id: review
    driver: agent
    on: [module.found]
    emits: [module.reviewed, module.skipped]
    state:
      key: "review.${event.payload.module}"
      skip_if: "${state.reviewed} == true"
    params:
      prompt_file: review.prompt.md
```

Fields:

- `key` (template). A stable, deterministic key for the item being processed.
  Templates can use `event.payload.*`, `event.shard`, `node.id`, `node.fires`, and
  `config.*`. The same key across runs maps to the same persisted record.
- `skip_if` (boolean template, optional). If the expression resolves to a truthy
  value, the engine does not invoke the driver. Instead it emits `node.skipped`
  (and any events the user has declared in `emits:` that are also emitted on
  skip, such as `module.skipped`). This makes re-runs idempotent and cheap.
- `store` (optional). Reference a named store. If omitted, the workflow's default
  state store is used.
- `inherit` (optional, default `true`). Whether to start `ctx.state` from the
  persisted record. When `false`, the node starts with an empty `StateStore` and
  overwrites any existing record on completion.

### 3. Execution semantics

Before a node fires:

1. The engine resolves the `state.key` template from the trigger event(s), node,
   and workflow config.
2. It loads the persisted record for that key (if any) into a `StateStore` view.
3. It evaluates `skip_if`. If true, it emits `node.skipped` and, where declared,
   the matching skip event, then stops.
4. Otherwise it runs the driver with `ctx.state` bound to that view.

After a successful driver invocation:

1. The engine writes the view back to the persistent `StateStore` under the
   resolved key.
2. The normal event emission continues, so the rest of the workflow can react to
   the outcome.

On failure or cancellation, the view is not written back, keeping the previous
persisted state intact and allowing the item to be retried on the next run.

### 4. Event state

Every event emitted from a node with a `state.key` will carry an optional
`state_key` field. This lets downstream nodes and observers correlate an event
with its persisted record. Template placeholders will support `${state.X}` to
read the current key's state and `${event.state_key}` to pass the key along.

### 5. Workflow-level defaults

A workflow may declare a top-level `state:` block that sets the default store and
key prefix for all nodes:

```yaml
name: module-review
state:
  store: json
  file: .ac/module-review.state.jsonl
```

Node-level `state:` values override the defaults.

## Consequences

- Exhaustive, item-by-item workflows become cacheable, resumable, and
  introspectable: an item is either `done`, `failed`, or `pending` in a plain
  JSONL file that can be read outside Peregrin.
- The `ctx.state` API already in `ExecutionContext` remains the same, so existing
  drivers and handlers are not broken.
- `--replay` can restore the `StateStore` before the run, and `--resume` can load
  it without replaying every event.
- Strict mode (ADR-0001) can validate `state.key` and `state.skip_if` templates
  at load time.
- New I/O is introduced per node with `state:`. Memory store keeps it cheap for
  `--dry-run` and `--no-store`.
- `ac-store-json` and `ac-store-memory` need to implement `StateStore`; the
  engine needs to wire the port and load/save state around node execution.

## Alternatives considered

- **Keep `ctx.state` purely in-memory.** Rejected because it does not support the
  cache/skip/resume use cases that motivated this feature.
- **Derive state from the `EventStore` log on every replay.** Rejected because
  replaying and folding every event to reconstruct per-item state is expensive,
  fragile, and makes it hard for users to edit or inspect progress.
- **Make state a mutable payload on events.** Rejected because events are the
  immutable audit log; mixing mutable state into the log would break the log's
  append-only guarantee and complicate `--replay` semantics.
