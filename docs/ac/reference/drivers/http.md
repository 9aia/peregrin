# http driver

Makes HTTP requests and maps status codes to emitted events.

## Basic usage

```yaml
- id: fetch-api
  driver: http
  on: [start]
  emits: [api.success, api.failed]
  with:
    method: GET
    url: "https://api.github.com/repos/${event.payload.repo}"
```

2xx emits `api.success`; non-2xx emits `api.failed`.

## Status code mapping

```yaml
- id: webhook
  driver: http
  on: [deploy.done]
  emits: [ok, not_found, server_error, other]
  with:
    method: POST
    url: "https://api.example.com/webhook"
    json:
      message: "deployed"
    emit:
      200: ok
      404: not_found
      500: server_error
      5xx: server_error
      default: other
```

## Auth

Bearer:

```yaml
with:
  url: "https://api.example.com/data"
  auth:
    type: bearer
    secret: API_TOKEN
```

Basic:

```yaml
with:
  auth:
    type: basic
    user: admin
    password_secret: DB_PASSWORD
```

Custom header:

```yaml
with:
  auth:
    type: header
    key: X-API-Key
    secret: API_KEY
```

## Query params, headers, and timeout

```yaml
with:
  method: GET
  url: "https://api.example.com/search"
  params:
    q: "test"
    page: 1
  headers:
    Accept: application/json
    X-Custom: my-value
  timeout: 30
```

## Response payload

| Field     | Type   | Description                                                      |
| --------- | ------ | ---------------------------------------------------------------- |
| `status`  | int    | HTTP status code                                                 |
| `body`    | any    | Response body (parsed JSON if Content-Type is JSON, else string) |
| `headers` | dict   | Response headers                                                 |
| `url`     | string | Final URL after redirects                                        |
| `method`  | string | HTTP method used                                                 |

Access downstream with `${event.payload.*}`.

## Realistic example

```yaml
name: pr-notify

nodes:
  - id: fetch-pr
    driver: http
    on: [pr.opened]
    emits: [pr.fetched, pr.fetch_failed]
    with:
      method: GET
      url: "https://api.github.com/repos/${event.payload.repo}/pulls/${event.payload.pr_number}"
      auth:
        type: bearer
        secret: GITHUB_TOKEN
      headers:
        Accept: application/vnd.github.v3+json

  - id: notify-slack
    driver: http
    on: [pr.fetched]
    emits: [notified]
    with:
      method: POST
      url: "https://hooks.slack.com/services/${secret.SLACK_WEBHOOK}"
      json:
        text: "New PR: ${event.payload.body.title} by ${event.payload.body.user.login}"

  - id: alert-failure
    driver: http
    on: [pr.fetch_failed]
    emits: [alerted]
    with:
      method: POST
      url: "https://hooks.slack.com/services/${secret.SLACK_WEBHOOK}"
      json:
        text: "Failed to fetch PR: HTTP ${event.payload.status}"
```

## See also

- [Streaming](streaming.md) — `with.stream` for HTTP
