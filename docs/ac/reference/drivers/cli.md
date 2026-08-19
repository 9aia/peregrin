# cli driver

Polls a shell command on an interval and emits one event per output line.

## Self-trigger mode (default)

```yaml
- id: poll-prs
  driver: cli
  on: [start, poll-prs.tick]
  emits: [pr.discovered, poll-prs.tick]
  max_fires: unlimited
  with:
    cmd: "gh pr list --json number,title --state open"
    interval: 60
    event_type: pr.discovered
```

The driver runs once, emits `<node-id>.tick` to re-trigger itself, and works
with `max_fires`, `until`, and cycle detection.

## Interval mode

```yaml
- id: poll-prs
  driver: cli
  on: [start]
  emits: [pr.discovered]
  with:
    cmd: "gh pr list --json number,title --state open"
    interval: 60
    mode: interval
    event_type: pr.discovered
```

Loops internally with `keep_alive`. No self-triggering needed.

## JSON output

```yaml
- id: poll-events
  driver: cli
  on: [start, poll-events.tick]
  emits: [alert, info, poll-events.tick]
  max_fires: unlimited
  with:
    cmd: "curl -s https://api.example.com/events"
    interval: 30
    parse_json: true
```

If an output line is valid JSON and has a `type` field, that type is used.

## Emitted events

- `<node-id>.listening` on start.
- One event per output line, with `rc` in the payload.
- `<node-id>.done` when the driver stops.

## Common `with:` fields

| Field        | Type   | Description                                  |
| ------------ | ------ | -------------------------------------------- |
| `cmd`        | string | Shell command to run                         |
| `interval`   | int    | Seconds between runs                         |
| `mode`       | string | `self_trigger` (default) or `interval`       |
| `event_type` | string | Event type per line (default: first `emits`) |
| `parse_json` | bool   | Parse each line as JSON                      |

## See also

- [Polling](polling.md) — the `poll:` sugar that works with any driver
