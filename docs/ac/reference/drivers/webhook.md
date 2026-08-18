# webhook driver

Starts an HTTP server. Each POST to the path emits an event. The driver
sets `runner.keep_alive = True` so the workflow does not quiesce while
waiting for requests.

## Basic usage

```yaml
- id: listen
  driver: webhook
  on: [start]
  emits: [webhook.received]
  with:
    port: 8080
    path: /events
    host: 127.0.0.1
    event_type_field: type
```

Binds to `127.0.0.1` by default. Use `host: 0.0.0.0` to expose to the network.

## Emitted events

- `<node-id>.listening` when the server starts.
- `<node-id>.received` (or whatever is first in `emits`) for each POST.

## Auth

### Shared secret (header)

```yaml
- id: listen
  driver: webhook
  on: [start]
  emits: [webhook.received]
  with:
    port: 8080
    path: /events
    auth:
      type: header
      name: X-API-Key
      secret: WEBHOOK_TOKEN
```

### HMAC signature

```yaml
- id: listen
  driver: webhook
  on: [start]
  emits: [webhook.received]
  with:
    port: 8080
    path: /webhook
    auth:
      type: hmac
      algorithm: sha256
      header: X-Signature-256
      secret: WEBHOOK_SECRET
      timestamp_header: X-Timestamp
      replay_window: 300
```

## TLS

```yaml
- id: listen
  driver: webhook
  on: [start]
  emits: [webhook.received]
  with:
    port: 8443
    path: /events
    tls:
      cert: "${secret.TLS_CERT}"
      key: "${secret.TLS_KEY}"
```

## Body size limit

```yaml
with:
  max_body_size: 1048576
```

Default is 1MB. Oversized requests are rejected with 413.

## Common `with:` fields

| Field              | Type   | Description                                          |
| ------------------ | ------ | ---------------------------------------------------- |
| `port`             | int    | TCP port                                             |
| `path`             | string | URL path                                             |
| `host`             | string | Bind address (default `127.0.0.1`)                   |
| `event_type_field` | string | JSON field to use as the event type (default `type`) |
| `auth`             | dict   | See auth sections above                              |
| `tls`              | dict   | `cert` and `key` PEM strings                         |
| `max_body_size`    | int    | Max body size in bytes                               |

## Health check

`GET /health` returns `{"status": "ok"}` — useful for Kubernetes
liveness/readiness probes.

## Capability enforcement

Webhook nodes are checked against `webhook` capabilities at dispatch time.
See [Safety](../../explanation/safety.md).

```yaml
capabilities:
  default_policy: deny
  webhook:
    ports: [8080, 8443]
    hosts: ["127.0.0.1"]
    paths: ["/events", "/webhook/*"]
    require_auth: true
```

## Stopping the server

Emit `workflow.cancel` from any node to stop the server and end the workflow.
