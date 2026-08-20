# Polling

Four ways to poll.

## 1. `poll:` sugar (recommended)

```yaml
- id: poll-prs
  driver: shell
  poll:
    interval: 60
    until: [pr.complete]
  emits: [pr.discovered]
  params:
    cmd: "gh pr list --json number,title --state open"
```

The `poll:` modifier expands to:

- `on: [start, <id>.tick]`
- `emits: [..., <id>.tick]`
- `until: [...]`

A `poll:` node without an explicit `max_fires` defaults to `1`. Add `max_fires: unlimited` (or a large integer) explicitly to poll forever.

- `params.interval` (if the driver supports it)
- After each successful run, the runner auto-emits `<id>.tick` delayed by
  `interval` seconds.

## 2. `driver: cli`

Purpose-built polling driver with per-line events and JSON parsing. See
[cli driver](cli.md).

## 3. `driver: shell` + self-trigger

```yaml
- id: poll-prs
  driver: shell
  on: [start, poll-prs.tick]
  emits: [pr.discovered, poll-prs.tick]
  max_fires: unlimited
  params:
    cmd: "sleep 60 && gh pr list --json number,title --state open"
```

## 4. `driver: http` + self-trigger

```yaml
- id: poll-api
  driver: http
  on: [start, poll-api.tick]
  emits: [api.response, poll-api.tick]
  max_fires: unlimited
  params:
    method: GET
    url: "https://api.example.com/status"
```

All approaches use `max_fires: unlimited` (unlimited) and `until: [some.event]`
to stop when a condition is met.

## `poll:` field reference

| Field      | Type | Description                                 |
| ---------- | ---- | ------------------------------------------- |
| `interval` | int  | Seconds between polls                       |
| `until`    | list | Stop polling once any of these events exist |
