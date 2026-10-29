+++
id = "ADR-0014"
title = "ADR-0014: Exhaust Node"
status = "open"
priority = "medium"
+++

# ADR-0014: Exhaust Node

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

ADR-0013 introduced persistent `state:` on nodes, making it possible to cache,
resume, and skip individual work items. The immediate use case is exhaustive
work: review every module, run every test, refactor every folder. With only the
`state:` primitive, the author still has to:

1. produce one trigger event per item (usually a shell loop with `stream: true`),
2. add a `state.key` template to every worker node,
3. add `state.skip_if` to every worker node,
4. track when all items are done, and
5. restart the workflow from the beginning each time to re-discover which items
   are still pending.

This is a lot of boilerplate for a very common pattern. A higher-level
abstraction that wraps the item list and the skip/done logic would make
exhaustive workflows as declarative as a normal node.

## Decision

Peregrin will add an `exhaust` node type that fans out a list of items while
honouring `state` so already-completed items are skipped.

### YAML surface

```yaml
nodes:
  - id: review_all
    driver: exhaust
    on: [start]
    emits: [module.to_review]
    exhaust:
      items: "${config.modules}"
      as: module
      emit: module.to_review
      done: module.reviewed
      state:
        key: "review.${event.payload.module}"
        skip_if: "${state.reviewed}"
```

Fields:

- `items` (template, required). Resolves to an array of strings or objects. If
  the elements are strings, the payload key is controlled by `as`. If the
  elements are objects, they are merged into the emitted payload directly.
- `as` (string, optional, default `item`). The payload key used for string
  `items`. With `as: module` the emitted event is `{ module: "packages/handl-core" }`.
- `emit` (string, required). The event type to emit for each pending item.
- `done` (string, optional). An event type that marks an item complete. When
  every item has produced a `done` event, `exhaust` stops and emits
  `exhaust.complete`.
- `state` (optional). Same `state:` block as a normal node. It is evaluated per
  item to decide whether to skip it.

### Execution semantics

1. On trigger, `exhaust` resolves `items` to an array.
2. For each element it computes the `state.key` and loads the persisted record.
3. If `state.skip_if` is truthy, the item is skipped and `exhaust.skipped` is
   emitted with `item` and `state_key`.
4. Otherwise it emits `emit` with the item in the payload.
5. It continues to re-evaluate on every matching `done` event (or on replay):
   - it re-emits `emit` for any item that is still pending;
   - it emits `exhaust.complete` when all items are done.

`exhaust` is itself a regular node, so it can declare `state` at the node level
if it needs to remember its own position across runs.

### Downstream worker

The worker node only needs to do the work and emit the `done` event:

```yaml
nodes:
  - id: review
    driver: agent
    on: [module.to_review]
    emits: [module.reviewed]
    params:
      runner: devin
      prompt: "Review ${event.payload.module}"
```

`module.reviewed` is the `done` event that `exhaust` watches.

### Built-in events

- `exhaust.skipped` — emitted when an item is skipped because `state.skip_if` is
  true.
- `exhaust.complete` — emitted when every item has produced a `done` event.
- `exhaust.progress` — optionally emitted after each `done` with
  `{ done: N, total: N, remaining: [...] }` for dashboards or humans.

## Consequences

- Exhaustive workflows become first-class and do not require shell loops,
  `stream: true`, or per-node `state.skip_if` boilerplate.
- The `exhaust` node re-uses the `state` primitive from ADR-0013, so it works
  with any `StateStore` backend (memory, JSON, future stores).
- `exhaust` nodes may fire multiple times per workflow (on replay or when new
  `done` events arrive), so the implementation must be idempotent and must not
  double-emit `emit` for an already-pending item.
- The engine needs a new `exhaust` driver or special handling in `Runner` for
  `driver: exhaust` nodes.
- `handl validate` and `handl typegen` need to understand `exhaust:` schema.
- The existing `state:` block on normal nodes stays unchanged and is still the
  right choice for one-off cacheable work.

## Alternatives considered

- **Keep only the `state:` primitive.** Rejected because the boilerplate of
  listing, emitting, and skipping per item is too high for such a common pattern.
- **Make `exhaust` a `runner`-style meta-node that injects a loop of nodes at
  runtime.** Rejected because it is harder to validate, visualise, and resume.
  A declarative `exhaust` field is simpler and more transparent.
- **Use `max_fires` and `until` to express exhaustion.** Rejected because those
  primitives control a single node's firing budget, not a list of items.
