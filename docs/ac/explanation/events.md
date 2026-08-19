# Events

Events are the unit of flow and state. Every event has a type (string), a payload (dict), and a source (node id). The append-only event log IS the workflow state.

## Built-in events

All built-in events come from `__runner__` (or the failing node). You can subscribe to any of them with `on:`.

- `start` — synthetic entry point. Every workflow begins here.
- `workflow.done` — emitted when the graph quiesces (no more ready nodes).
- `workflow.cancel` — emitted by a driver via `ctx.emit(new Event("workflow.cancel"))`.
- `workflow.cancelled` — emitted by the runner when it processes a cancel.
- `workflow.failed` — emitted instead of `workflow.done` when any node fails.
- `workflow.cycle_detected` — emitted when cycle detection triggers.
- `node.failed` — per-node failure. Source is the failing node id.
- `event.invalid` — emitted when an event's payload fails schema validation.
- `dlq.event` — emitted when a failed event goes to the dead-letter queue.
- `circuit.open` — emitted when a circuit breaker trips.
- `<node-id>.listening` — emitted by long-lived drivers (webhook, stdin, cli) when they start.
- `<node-id>.done` — emitted by long-lived drivers when they complete (e.g. EOF on stdin).

## Custom events

Custom events are just strings. No registration is required.

### Naming convention

Dot-separated, hierarchical: `pr.approved`, `deploy.success`, `shard.3.done`.

This enables wildcards: `on: [pr.*]` catches `pr.approved`, `pr.rejected`, etc.

### Emitting custom events

From a shell node:

```yaml
- id: review
  driver: shell
  on: [pr.opened]
  emits: [pr.approved, pr.changes_requested]
  with:
    cmd: "claude --print 'Review: ${event.payload.url}'"
```

From a TypeScript driver:

```ts
import { Effect } from "effect";
import { Driver, Event } from "ac-core";

const ReviewDriver: Driver = {
  name: "review",
  invoke: (node, triggers, _ctx) =>
    Effect.gen(function* () {
      const pr = triggers[0].payload;
      const comments = "..."; // call an LLM
      if (comments.includes("LGTM")) {
        return [new Event("pr.approved", { pr: pr.number, comments }, node.id)];
      }
      return [new Event("pr.changes_requested", { pr: pr.number, comments }, node.id)];
    }),
};
```

## Why events are the state

Because the log is append-only and every action is recorded, the entire workflow state can be reconstructed by replaying the log. This is what makes crash recovery, observability, and debugging straightforward.

## See also

- [Event schemas](../reference/event-schemas.md) — runtime payload validation
- [Wildcard matching](../reference/wildcard-matching.md) — pattern syntax
- [Write a workflow](../tutorials/write-a-workflow.md) — using events in nodes
- [Drivers](drivers.md) — what code can emit events
