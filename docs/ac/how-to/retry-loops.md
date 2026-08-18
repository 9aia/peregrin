# Write retry and polling loops

Some loops are intentional (retry, refinement, polling). Use `max_fires` and
`until` to bound them. Accidental loops are caught by cycle detection.

## Retry N times

```yaml
nodes:
  - id: attempt
    driver: shell
    on: [start, op.retry]
    emits: [op.success, op.retry]
    max_fires: 5
    until: [op.success]
    with:
      emit:
        0: op.success
        default: op.retry
      cmd: "deploy.sh"
```

- `max_fires: 5` — the node fires at most 5 times.
- `until: [op.success]` — stops firing once `op.success` exists in the log.
- `with.emit` — maps exit codes to events.

## Poll until condition

```yaml
nodes:
  - id: poll
    driver: shell
    on: [start, poll.tick]
    emits: [poll.ready, poll.tick]
    max_fires: 0
    until: [poll.ready]
    with:
      emit:
        0: poll.ready
        default: poll.tick
      cmd: "check-status.sh"
```

## Refinement loop (bounded + conditional)

```yaml
nodes:
  - id: refine
    driver: python
    handler: myapp:refine
    on: [start, refine.iter]
    emits: [refine.good, refine.iter]
    max_fires: 10
    until: [refine.good]
```

## `max_fires` vs `until` vs `retire_on`

| Field                | What it does                     | Use case                  |
| -------------------- | -------------------------------- | ------------------------- |
| `max_fires: N`       | Bounds total fires to N          | Retry N times             |
| `until: [event]`     | Stops firing once event exists   | Stop on success/condition |
| `retire_on: [event]` | Removes node from graph entirely | Clean up injected nodes   |

## Accidental loops

If you forget to bound a loop, the runner detects repeating event sequences
and aborts:

```
cycle detected: last 20 events repeated 3 times — aborting
```

Disable with `--no-cycle-detection` or `cycle_detection=False`.

## See also

- [Loop protection](../explanation/loop-protection.md)
- [Write a workflow](../tutorials/write-a-workflow.md)
- [Self-modifying workflows](../explanation/self-modifying.md)
