# Streaming (shell and http)

Both `shell` and `http` drivers support `with.stream: true` — emit one
event per line or chunk as output arrives, instead of waiting for completion.

## Shell streaming

```yaml
- id: tail-logs
  driver: shell
  on: [start]
  emits: [log.line]
  with:
    cmd: "tail -f /var/log/app.log"
    stream: true
```

Emits `log.line` (the first declared `emits`) per line of stdout. Emits
`<node-id>.done` with the exit code when the process exits.

## HTTP streaming (SSE / NDJSON)

```yaml
- id: listen-sse
  driver: http
  on: [start]
  emits: [sse.event]
  with:
    method: GET
    url: "https://api.example.com/events/stream"
    stream: true
```

Emits `sse.event` (first declared `emits`) per line of the response body.
If the line is valid JSON, the payload contains `{"data": <parsed>, "raw": <line>}`.
Otherwise, the payload is `{"line": <text>}`. Emits `<node-id>.done` with
the status code when the response completes.

## Emitted events

- One event per line/chunk, using the first declared `emits` value.
- `<node-id>.done` when the stream ends (shell exit or HTTP response close).

## Common `with:` fields

| Field    | Type | Description      |
| -------- | ---- | ---------------- |
| `stream` | bool | Enable streaming |

## See also

- [shell driver](shell.md)
- [http driver](http.md)
