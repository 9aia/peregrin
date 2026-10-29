+++
id = "ADR-0050"
title = "ADR-0012: HMR Module"
status = "open"
priority = "medium"
+++

# ADR-0012: HMR Module

## Status

Superseded by [handl ADR-0021](../../packages/handl/project/adrs/0021-hmr.md). HMR is now a language toolchain concern. Peregrin may extend it with orchestration-specific state preservation.

Proposed

## Context

Peregrin workflows are YAML files loaded by `handl-loader-yaml` and executed by the
`handl-engine` `Runner`. Today, changing a workflow while it is running requires
stopping the process and starting it again. This is acceptable for short-lived,
CI-style workflows but is painful for long-running workflows such as polling,
streaming, and human-in-the-loop campaigns, where you want to iterate without
losing the in-flight campaign, the event log, or pending events.

The engine already has primitives that can support in-place updates:
`EventBus.inject` adds nodes at runtime, and nodes can be retired with `retireOn`.
What is missing is a user-facing, dev-server-driven mechanism that watches the
YAML, computes a diff, and applies it safely while letting the workflow author
decide how to preserve or tear down state.

Vite's HMR design provides a proven mental model: a WebSocket channel between
dev server and runtime, and explicit lifecycle hooks (`accept`, `dispose`,
`decline`, `invalidate`, `prune`) that let a module control what happens when it
changes. We will adapt that model to Peregrin YAML workflows.

## Decision

Peregrin will add an HMR module (`packages/handl-hmr`) and a new `handl dev` CLI
command. The module provides a dev server that watches workflow files, computes
a minimal update delta, and sends it to the runtime over a WebSocket. The
runtime applies the delta by retiring, injecting, and replacing nodes through the
existing `EventBus` while honouring lifecycle hooks declared in the YAML.

### Components

```text
┌──────────────┐     file change     ┌──────────────┐
│  YAML +      │  ────────────────▶  │  HmrServer   │
│  includes    │                     │  (dev tool)  │
└──────────────┘                     └──────┬───────┘
                                            │ WebSocket
                                            ▼
                                     ┌──────────────┐
                                     │  HmrClient   │
                                     │  (runtime)   │
                                     └──────┬───────┘
                                            │
                                            ▼
                                     ┌──────────────┐
                                     │   EventBus   │
                                     │  (inject /   │
                                     │   retire)    │
                                     └──────────────┘
```

- **`HmrServer`** (dev server). Watches the workflow file and any `include:`
  paths. On change, reloads the YAML through `handl-loader-yaml`, computes a minimal
  diff (`added`, `removed`, `changed` nodes and `config` changes), and sends it
  to the runtime. Falls back to `full-reload` when a change cannot be applied
  safely.
- **`HmrClient`** (runtime). Receives WebSocket messages inside the runner
  process, validates them with a `Schema`, and forwards them to
  `HmrUpdateHandler`.
- **`HmrUpdateHandler`**. Applies the diff to the running engine. It retires
  removed nodes, injects new nodes, and replaces changed nodes for which the
  workflow author has opted in. It emits the lifecycle events defined in the
  `hmr` section before and after each operation.
- **`HmrUpdate` schema**. A typed JSON message describing the delta. It is the
  boundary contract between dev server and runtime.

### WebSocket protocol (proposed)

Messages are JSON over WebSocket.

Client to server (optional):

- `{ "type": "hmr:ready" }`
- `{ "type": "hmr:custom", "payload": ... }`

Server to client:

- `{ "type": "hmr:update", "added": [...], "removed": [...], "changed": [...], "config": {...} }`
- `{ "type": "hmr:full-reload", "reason": "..." }`
- `{ "type": "hmr:error", "error": "..." }`

### YAML surface (proposed)

`hmr` may appear at the workflow level and on individual nodes. Node-level
settings override workflow-level defaults.

Workflow-level example:

```yaml
hmr:
  accept: [poller, worker]
  before: emit: workflow.pause
  after: emit: workflow.resume
  on_error: full-reload
```

Node-level example:

```yaml
nodes:
  - id: worker
    driver: shell
    on: [start, worker.tick]
    emits: [worker.done]
    hmr:
      accept: true
      before: emit: worker.pause
      after: emit: worker.resume
      dispose: emit: worker.cleanup
      data:
        counter: from.state
```

Fields:

- `accept` (boolean or list of node ids). Whether the changed node can be
  replaced in place. If `false` or if the node id is not in a workflow-level
  accept list, the dev server will fall back to `full-reload`.
- `decline` (list of node ids). A shorthand for nodes that always force a
  `full-reload` when changed.
- `before` / `after`. Events to emit before and after the HMR delta is applied.
  This lets the workflow pause I/O, flush buffers, or re-arm listeners around
  the swap.
- `dispose`. Events to emit before an old node definition is removed or replaced
  (similar to `import.meta.hot.dispose`). Used to persist or clean up state.
- `prune`. Events to emit after a node is removed from the workflow (similar to
  `import.meta.hot.prune`).
- `data`. A map of values to carry across updates for the same node id (similar
  to `import.meta.hot.data`). Values can be read from `ctx.state` or the event
  log and written back on `dispose`.
- `invalidate`. A list of downstream node ids that must also be reloaded when
  this node changes.
- `on_error` (workflow-level only). One of `full-reload`, `cancel`, or `ignore`.
  Determines what happens when the dev server cannot safely apply a change.

### Lifecycle

1. The dev server detects a file change.
2. It reloads the workflow, validates it, and computes a diff.
3. If any changed or removed node is not accepted for HMR, the server sends
   `hmr:full-reload` and the runtime restarts the campaign from the event log.
4. For an accepted delta:
   - Emit `before` events.
   - For removed nodes, emit `dispose` and `prune`, then retire them.
   - For changed nodes, emit `dispose`, retire the old definition, inject the
     new definition, and restore any `data` values.
   - For new nodes, inject them.
   - Emit `after` events.

## Consequences

- Long-running workflows can be edited and iterated on without a full process
  restart, matching the developer experience of Vite.
- The `EventBus` `inject` and `retireOn` primitives become first-class parts of
  the HMR contract. New `HmrUpdateHandler` may require additional `EventBus`
  methods to replace a node while preserving `fireCount` for unchanged parts.
- Workflow authors get explicit control over state preservation and teardown
  through YAML hooks, avoiding surprise state loss.
- The dev server adds a WebSocket dependency. It must only be used in local
  development; remote HMR is out of scope for the first version.
- Invalidation and full-reload must be carefully defined so that a partially
  applied update does not leave the campaign in an inconsistent state.

## Alternatives considered

- **Restart the process on every change.** Rejected because it loses the
  in-flight campaign and pending events, which is exactly the pain point for
  long-running workflows.
- **Watch the filesystem inside the runner and reload automatically.** Rejected
  because it couples the engine to file watching and removes the author's ability
  to control the update lifecycle. A separate dev server keeps the engine pure.
- **Use a generic file diff and always apply it.** Rejected because not every
  change is safe to apply blindly. Hooks like `accept`, `dispose`, and `prune`
  are needed for correctness.
