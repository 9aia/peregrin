# ADR-0027: `replayable` flag

## Status

Proposed

## Context

A workflow can be replayed only if every step produces the same output for the same input. Some operations are inherently non-deterministic: random values, UUIDs, timestamps, HTTP POSTs that create side effects, human approvals, or shell commands that read external state. Others are deterministic: pure computations, HTTP GETs that are cached, or shell commands against a known working tree.

Today the engine has no explicit concept of replayability. A workflow author cannot declare that a node is safe or unsafe to replay. The template engine will soon support non-deterministic functions (`random`, `uuid`, `now`), custom functions via YAML, and custom drivers via YAML. Without a `replayable` flag, the runner cannot know whether a replay is faithful to the original run.

## Decision

Add a `replayable` boolean that can be set at multiple levels. The effective value for a workflow is `true` only if every relevant component is replayable.

- `Workflow.replayable` defaults to `true` and is the final computed flag.
- `Node.replayable` is optional. If set, it overrides the automatic inference for that node.
- `Driver.replayable` is part of the `Driver` metadata. Built-in drivers declare their own default (e.g. `http` is replayable only for GET/HEAD if idempotent; `human` is not replayable; `shell` is not replayable unless explicitly configured).
- `DriverPreset.replayable` is optional and can override the base driver. It defaults to the base driver's value.
- `Function.replayable` is part of the function metadata. Built-in non-deterministic functions set `replayable: false`; custom functions via YAML can declare it.

The effective `replayable` flag is computed by the runner when a workflow is loaded:

1. Start with `Workflow.replayable = true` (the default).
2. For each node in the workflow:
   - If `Node.replayable` is explicitly set, use it.
   - Otherwise, resolve the node driver. If the driver or any function in the node's `params` is non-replayable, the node is non-replayable.
   - If any node is non-replayable, `Workflow.replayable` becomes `false`.
3. The computed value is exposed in the workflow metadata and the runner warns when replay is attempted on a non-replayable workflow.

Replays of non-replayable workflows use a "record and re-emit" strategy: the original output of each non-replayable node is captured in the event store and replayed as an event rather than re-executed. The runner skips the driver invocation for that node and emits the recorded event instead.

## Consequences

- Workflows gain an explicit replay contract.
- Authors can override replayability for a node when they know the operation is safe or unsafe.
- The event store must persist the output of non-replayable nodes so they can be replayed as events.
- Strict mode can enforce that every non-replayable node is explicitly marked, preventing accidental non-determinism.
- The CLI can show `replayable: true|false` in workflow summaries and block `peregrin replay` for non-replayable workflows unless `--allow-non-replayable` is set.

## Alternatives considered

- **Replayable only at workflow level.** Rejected. It would hide the source of non-determinism and prevent node-level overrides.
- **Infer replayability solely from function names.** Rejected. The same function can be deterministic if seeded, and some built-in drivers (like `http`) are conditionally replayable.
- **No replay support at all for non-deterministic workflows.** Rejected. Users still want to rerun or resume workflows; recording events is a practical middle ground.
