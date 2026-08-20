# stdin driver

Reads lines from stdin and emits one event per line. Sets `keep_alive = True`
so the workflow does not quiesce while waiting for input. On EOF, emits
`<node-id>.done` and clears `keep_alive`.

## Basic usage

```yaml
- id: read
  driver: stdin
  on: [start]
  emits: [line.received]
  params:
    event_type: line.received
    parse_json: false
```

## Plain text mode

Default: each line becomes `{"line": "<text>"}`. The `event_type` defaults
to the first declared `emits` value.

## JSON mode

```yaml
- id: read
  driver: stdin
  on: [start, alert, info]
  emits: [alert, info]
  params:
    parse_json: true
```

Each line is parsed as JSON. If the parsed object has a `type` field, it is
used as the event type (and removed from the payload). Otherwise, the
configured `event_type` is used.

## Emitted events

- `<node-id>.listening` when the driver starts.
- `<node-id>.done` on EOF.
- One event per line, using the configured or inferred type.

## Common `params:` fields

| Field        | Type   | Description                                       |
| ------------ | ------ | ------------------------------------------------- |
| `event_type` | string | Event type for each line (default: first `emits`) |
| `parse_json` | bool   | Parse each line as JSON                           |

## See also

- [Polling](polling.md)
