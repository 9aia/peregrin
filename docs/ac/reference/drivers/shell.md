# shell driver

Runs a command, maps exit code to emitted events.

## Basic usage

```yaml
- id: lint
  driver: shell
  on: [start]
  emits: [lint.passed, lint.failed]
  params:
    cmd: "ruff check ."
```

`rc==0` emits the first event (`lint.passed`). `rc!=0` emits the second
(`lint.failed`).

## Explicit exit-code mapping

```yaml
- id: deploy
  driver: shell
  on: [start]
  emits: [deploy.success, deploy.retry, deploy.failed]
  params:
    emit:
      0: deploy.success
      1: deploy.retry
      default: deploy.failed
    cmd: "deploy.sh"
```

## Environment, secrets, and timeout

```yaml
- id: deploy
  driver: shell
  on: [start]
  emits: [deploy.done]
  params:
    cmd: "deploy.sh"
    env:
      REGION: us-east-1
    secrets: [API_KEY, DB_PASSWORD]
    timeout: 300
```

## Common `params:` fields

| Field     | Type   | Description                                                    |
| --------- | ------ | -------------------------------------------------------------- |
| `cmd`     | string | Shell command to run                                           |
| `emit`    | dict   | Optional explicit exit-code → event mapping                    |
| `env`     | dict   | Environment variables for the command                          |
| `secrets` | list   | Secret names to inject as environment variables                |
| `timeout` | int    | Seconds before the command is killed                           |
| `stream`  | bool   | Emit one event per stdout line. See [streaming](streaming.md). |

## See also

- [Polling](polling.md) — polling patterns with any driver
- [Streaming](streaming.md) — `params.stream` on shell and http
