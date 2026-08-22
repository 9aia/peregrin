# ADR-0018: Temporal scheduling drivers

## Status

Proposed

## Context

Peregrin has one timer primitive today: `poll:`, which makes a node re-run on an interval until a stop condition is met. Many workflows need other temporal controls: a fixed delay, a pause until a wall-clock time, or a cron-style schedule. Examples include waiting for a deployment cooldown, scheduling a daily report, or giving a human gate business-hour boundaries.

The current workarounds are `poll:` with a long interval, `driver: shell` with `sleep`, or an external cron job invoking the Peregrin CLI. Each has drawbacks: `poll` is busy, `sleep` blocks the worker, and external cron loses the event graph.

## Decision

Introduce two built-in drivers: `delay` and `schedule`.

### `delay` driver

```yaml
- id: cooldown
  driver: delay
  on: [deploy.failed]
  emits: [cooldown.done]
  params:
    duration: 5m
```

`delay` waits for the specified duration, then emits `emits[0]` with `duration` and `until` metadata. It must not block the engine's concurrency pool; the engine schedules the future event and continues processing other ready nodes.

### `schedule` driver

```yaml
- id: daily-report
  driver: schedule
  on: [start]
  emits: [report.due]
  params:
    cron: "0 9 * * *"
    timezone: America/New_York
```

`schedule` emits `report.due` at the specified cron time. It can be one-shot, recurring, or bounded by `max_fires` and `until`.

Both drivers are compatible with `max_fires`, `until`, and `retire_on`.

## Consequences

- Time is represented as events in the graph, preserving the append-only event log.
- Workflows can express cooldowns, deadlines, and cron jobs without external tools.
- The engine needs an internal scheduler or timer queue to wake delayed events.
- A new package, `ac-driver-schedule`, is needed.
- `--dry-run` should short-circuit delays or print the scheduled time rather than actually waiting.

## Alternatives considered

- **Use `poll:` with a long `interval`.** Rejected because it repeatedly invokes the driver and is busy-wait in spirit.
- **Use `driver: shell` with `sleep`.** Rejected because it ties up a worker and is not observable as a scheduled event.
- **Use an external cron to trigger workflows via `webhook`.** Rejected because it moves the schedule out of the workflow file and makes the graph depend on external state.
