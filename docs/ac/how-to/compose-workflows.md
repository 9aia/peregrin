# How to compose workflows

Peregrin is built around composition: small fragments, independent nodes, and reusable runtime pieces combine into larger workflows. This guide shows which mechanism to use for each common composition task.

## Reuse a node definition

Use `include: path/to/file.yaml` to pull a whole node (or config, event schema, or `params` block) into your workflow. The included file is deep-merged into the parent mapping, so local values override shared ones.

```yaml
nodes:
  - include: lib/git/worktree.node.yml
  - id: build
    driver: shell
    on: [git.worktree.ready]
    emits: [build.done]
    params:
      cmd: "npm run build"
```

## Inherit from a named template

Use `snippet: name` to merge a named fragment into any dict. This is the name-based equivalent of `include:` and is useful when a fragment is referenced from many places.

```yaml
snippets:
  shell:
    driver: shell
    params:
      timeout: 60

nodes:
  - id: lint
    snippet: shell
    on: [start]
    emits: [lint.done]
    params:
      cmd: "ruff check ." # overrides the snippet's with
```

## Inject a value at a specific position

Use `${include(path)}` or `${snippet(name)}` when you need the file or snippet placed at a single YAML value, not merged into the parent dict. They work for scalars, lists, and dicts.

```yaml
nodes:
  - id: deploy
    driver: shell
    on: [build.done]
    params:
      env: ${include(env.yaml)}
      cmd: "deploy --region=${include(region.yaml)}"
      on: ${snippet(pr-triggers)}
```

## Auto-discover a local library

Put files in `.handl/` next to the workflow (or alongside it). The file router auto-discovers components by suffix, so you can keep node definitions, snippets, handlers, and schemas together by domain.

```
my-workflow.yaml
.handl/
  git/
    worktree.node.yml
    worktree.handler.py
    worktree.event.json
```

```yaml
name: my-workflow

nodes: [] # nodes are auto-loaded from .handl/**/*.node.yml
```

## Wait for parallel branches

Use `mode: all` to fire a node only after every pattern in `on:` has occurred. For a dynamic number of matching events, use `fan_in:` with `count` or `count_from`.

```yaml
- id: summarize
  driver: shell
  on: [lint.passed, test.passed]
  mode: all
  emits: [summary.done]
  params:
    cmd: "echo 'lint and test both passed'"
```

```yaml
- id: wave-complete
  on: [pr.finished]
  emits: [wave.done]
  fan_in:
    count_from: config.wave.prs
    emit: wave.done
```

## Fan out to dynamic shards

Use `*` and `**` wildcards in `on` and `emits` to handle a number of shards that is not known in advance. Capture shard IDs with `${event.shard}`.

```yaml
- id: worker
  driver: shell
  on: [shard.*.ready]
  emits: [shard.*.done]
  max_fires: unlimited
  params:
    cmd: "process ${event.shard}"
```

## Forward events without a bridge node

Use `forward:` to re-emit a matching trigger under one or more new event types. This removes the need for thin orchestrator shell nodes that just echo events.

```yaml
- id: ci-bridge
  on: [lint.*, test.*]
  forward: [ci.announce]
```

## Compose safety boundaries

Workflow `capabilities` define the outer security boundary. Node `capabilities` can only narrow that boundary, never widen it. The composed boundary is checked at dispatch time, before the driver runs.

```yaml
capabilities:
  default_policy: deny
  shell:
    allow: ["echo *", "git *"]

nodes:
  - id: safe
    driver: shell
    capabilities:
      shell:
        allow: ["echo *"]
    params:
      cmd: "echo hello" # allowed

  - id: unsafe
    driver: shell
    params:
      cmd: "rm -rf /" # blocked
```

## Wire the runtime in code

For tests or a custom CLI, compose the engine with Effect `Layer`s. Each layer provides a service; `Layer.merge` combines parallel requirements and `Layer.provide` satisfies them.

```ts
import { Effect } from "effect";
import { RunnerLiveLayer, EventBusLiveLayer, DriverRegistryLiveLayer } from "handl-engine";
import { Live as MemoryStoreLayer } from "handl-store-memory";

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  return yield* runner.run(workflow);
}).pipe(
  Effect.provide(RunnerLiveLayer),
  Effect.provide(EventBusLiveLayer),
  Effect.provide(DriverRegistryLiveLayer),
  Effect.provide(MemoryStoreLayer),
);
```

## When to use which

| Task                            | Mechanism                       | See also                                                |
| ------------------------------- | ------------------------------- | ------------------------------------------------------- |
| Share a node definition         | `include:` or file router       | [Compose from fragments](compose-from-fragments)        |
| Template a node                 | `snippet:` / `${snippet()}`     | [Compose from fragments](compose-from-fragments)        |
| Place a file's value in one key | `${include()}` / `${snippet()}` | [Include resolution](../explanation/include-resolution) |
| Wait for all branches           | `mode: all` or `fan_in:`        | [Full feature list](../reference/features)              |
| Handle dynamic shards           | `*` / `**` wildcards            | [Wildcard matching](../reference/wildcard-matching)     |
| Re-emit an event                | `forward:`                      | [Full feature list](../reference/features)              |
| Restrict what a node can do     | `capabilities:`                 | [Safety](../explanation/safety)                         |
| Compose the runtime             | `Layer.merge` / `Layer.provide` | [Programmatic API](programmatic-api)                    |
