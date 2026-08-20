# Dynamic fan-out (map-reduce)

A handler emits N events at runtime; wildcard subscribers catch each.

## Pattern

```yaml
nodes:
  - id: planner
    driver: python
    handler: myapp:plan_shards
    on: [start]
    emits: [shard.*.ready]

  - id: worker
    driver: shell
    on: [shard.*.ready]
    emits: [shard.done]
    max_fires: unlimited
    params:
      cmd: "process ${event.payload.id}"

  - id: reduce
    driver: shell
    on: [shard.**.done]
    mode: all
    emits: [reduce.done]
    params:
      cmd: "echo 'all shards done'"
```

## How it works

1. `planner` emits `shard.1.ready`, `shard.2.ready`, `shard.3.ready` at runtime.
2. `worker` has `on: [shard.*.ready]` — `*` matches one segment, so it catches each.
3. `max_fires: unlimited` means the worker can fire unlimited times (once per shard).
4. `reduce` has `on: [shard.**.done]` with `mode: all` — waits for all shards to finish.

## See also

- [Wildcard matching](../reference/wildcard-matching.md) — pattern syntax
- [Write a workflow](../tutorials/write-a-workflow.md)
- [Write retry and polling loops](retry-loops.md)
