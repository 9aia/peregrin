+++
id = "ADR-0021"
title = "ADR-0021: Map and reduce drivers"
status = "open"
priority = "medium"
+++

# ADR-0021: Map and reduce drivers

> **Platform note:** Peregrin is an orchestration plugin built on handl.
> handl provides the language, typechecker, module system, standard library,
> formatter, linter, package manager, and HMR. This ADR covers Peregrin-specific
> orchestration concerns only. Language-level features referenced here are
> defined in `packages/handl/project/adrs/`.


## Status

Proposed

## Context

Peregrin workflows often need to run the same task over a variable number of items: migrate N plugins, review N files, test N shards. The current patterns for this are powerful but require the author to hand-wire fan-out and fan-in.

`packages/handl-examples/examples/map-reduce.yaml` uses a `python` planner to emit `shard.1.ready`, `shard.2.ready`, `shard.3.ready` and hard-codes the emits list. The worker then uses `max_fires: 0` and a wildcard `on` to fire per shard, and the reducer waits on `shard.**.done` with `mode: all`.

`packages/handl-examples/examples/lib/campaign/shard-dispatcher.node.yml` improves fan-out with a Python `dispatch_shards` handler, and `packages/handl-examples/examples/lib/campaign/wave-fanin.node.yml` improves fan-in with `fan_in: {count_from: ...}`. But the author still needs two lib nodes, a Python handler, and a `config` convention to coordinate them.

These patterns prove the need exists. They should become first-class, declarative drivers.

## Decision

Introduce two built-in drivers: `map` and `reduce`.

### `map` driver

`map` consumes a list value and emits one event per item. It also registers the set of expected keys so `reduce` can know when everything is done.

```yaml
- id: dispatch
  driver: map
  on: [start]
  emits: [shard.*.start]
  params:
    from: config.plugins
    event: shard.{item}.start
    payload:
      id: "{item}"
```

This emits `shard.auth.start`, `shard.billing.start`, etc.

### `reduce` driver

`reduce` waits for one terminal event per item produced by a named `map` node, then emits an aggregate event.

```yaml
- id: collect
  driver: reduce
  on: [shard.*.done, shard.*.failed]
  emits: [migration.complete, migration.partial]
  params:
    source: dispatch
    emit_when:
      all_succeeded: migration.complete
      any_failed: migration.partial
    aggregate: votes
```

The `source` field tells the engine which `map` node produced the items. The engine tracks the keys emitted by `map` and counts `shard.<key>.done` and `shard.<key>.failed` events until every key has produced one terminal event.

## Consequences

- No Python handler is needed for basic sharded pipelines.
- No `max_fires: 0` workaround is needed.
- No `fan_in.count_from` path is needed.
- `reduce` can distinguish "all succeeded" from "any failed" and carry an aggregate payload.
- The engine must track the mapping between `map` output events and their source keys.
- A new package, `handl-driver-map-reduce`, is needed.

## Alternatives considered

- **Keep `fan_in` and `runner` injection.** This keeps the engine simpler, but every dynamic pipeline requires a Python handler and manual `fan_in` wiring.
- **Add a `for_each` field to every node.** This would fan the node itself, but it complicates the `on`/`emits` semantics and does not solve aggregation.
- **Implement `map` as a `runner` injection.** This gives observability, but it requires the `runner` driver and a `fan_in` tally, which is the current workaround we are replacing.
