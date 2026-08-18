# Loop protection

Three layers, from specific to general.

## 1. Per-node: max_fires + until + retire_on

**`max_fires`** — bounds how many times a node can fire. Default 1 (fire
once). Set to 0 for unlimited. Set to N for bounded retries.

This is the primary mechanism for intentional loops. A retry node with
`max_fires: 5` will fire at most 5 times, then stops — regardless of what
events are still flowing.

When a node hits its `max_fires` limit and another matching event
arrives, the bus emits a `node.exhausted` event (at most once per node
per run). This lets workflows react to exhaustion instead of silently
quiescing to `workflow.done`, which is ambiguous (neither "complete" nor
"failed"). A node that is also blocked by its `until` condition does not
emit `node.exhausted`; that is the intended stop, not exhaustion.

```yaml
- id: fix
  driver: agent
  on: [review.rejected]
  emits: [fix.done, fix.failed]
  max_fires: 3

- id: fix-exhausted
  driver: shell
  on: [node.exhausted]
  emits: [workflow.aborted]
  with:
    cmd: |
      echo "FIX EXHAUSTED — node '${event.payload.node}'"
      echo "  hit max_fires=${event.payload.max_fires}"
```

**`until`** — declarative termination. Stops a node from firing once any
listed event type exists in the log. "Keep going until done."

```yaml
max_fires: 5
until: [op.success]
```

**`retire_on`** — removes the node from the graph entirely. Unlike `until`
(which leaves the node in the graph but stops it from firing), `retire_on`
deletes it. For scoping injected nodes that would otherwise accumulate
forever.

## 2. Cycle detection (automatic)

The runner watches the event log. If the last N events (default 20) repeat
K times (default 3) in a row, the workflow is aborted with a
`workflow.cycle_detected` event.

This catches accidental infinite loops — the kind you did not intend to
create. A ping-pong loop with no bounds will be detected and stopped.

It does NOT fire on normal workflows, because normal workflows do not
repeat the same event sequence 3+ times in a row.

Configurable:

```bash
ac run workflow.yaml --cycle-window 20 --cycle-threshold 3
ac run workflow.yaml --no-cycle-detection
```

## 3. Global safety nets

**`max_events`** (default 10000) — hard stop on total event count.

**`timeout`** (default 0 = no limit) — wall-clock timeout in seconds.

```bash
ac run workflow.yaml --timeout 60 --max-events 1000
```

## Intentional vs accidental

| Type                   | How to express                           | Protection              |
| ---------------------- | ---------------------------------------- | ----------------------- |
| Retry N times          | `max_fires: N` + `on: [node.exhausted]`  | `max_fires`             |
| Poll until condition   | `max_fires: 0, until: [condition.event]` | `until`                 |
| Refinement loop        | `max_fires: N, until: [good.enough]`     | both                    |
| Clean up injected node | `retire_on: [done.event]`                | `retire_on`             |
| Accidental cycle       | (none — you forgot)                      | cycle detection         |
| Runaway production     | (none — handler is slow/stuck)           | `timeout`, `max_events` |

The principle: **express what you want explicitly; the runner catches what
you did not.**
