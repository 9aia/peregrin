# Write a workflow

A workflow is a YAML file with `nodes`. Each node subscribes to events,
runs a handler via a driver, and emits new events.

## Minimal workflow

```yaml
name: my-workflow
nodes:
  - id: do-thing
    driver: shell
    on: [start]
    emits: [thing.done]
    with:
      cmd: "echo 'doing the thing'"
```

Every workflow starts with a synthetic `start` event. Nodes with `on: [start]`
fire immediately.

## Parallel fan-out

Multiple nodes on the same event run in parallel automatically:

```yaml
nodes:
  - id: lint
    driver: shell
    on: [start]
    emits: [lint.passed, lint.failed]
    with:
      cmd: "ruff check ."

  - id: test
    driver: shell
    on: [start]
    emits: [test.passed, test.failed]
    with:
      cmd: "pytest -q"
```

## Fan-in (wait for multiple events)

`mode: all` — fire only after every `on` pattern has at least one match:

```yaml
- id: summarize
  driver: shell
  on: [lint.passed, test.passed]
  mode: all
  emits: [summary.done]
  with:
    cmd: "echo 'all passed'"
```

## Branching

`mode: any` (default) — fire on first matching event. Use multiple nodes
for branching:

```yaml
- id: notify-success
  on: [summary.done]
  emits: [notify.done]
  with:
    cmd: "echo 'success'"

- id: notify-failure
  on: [lint.failed, test.failed]
  mode: any
  emits: [notify.done]
  with:
    cmd: "echo 'failed'"
```

## Shell driver: exit codes → events

`emits: [A, B]` → rc==0 emits `A`, rc!=0 emits `B`:

```yaml
- id: deploy
  driver: shell
  on: [start]
  emits: [deploy.success, deploy.failed]
  with:
    cmd: "deploy.sh"
```

Explicit exit-code mapping with `with.emit`:

```yaml
- id: deploy
  driver: shell
  on: [start]
  emits: [deploy.success, deploy.retry, deploy.failed]
  with:
    emit:
      0: deploy.success
      1: deploy.retry
      default: deploy.failed
    cmd: "deploy.sh"
```

## Payload placeholders

All `with` fields support `${...}` substitution, powered by [lodash-es `_.template`](https://lodash.com/docs/4.17.15#template). Expressions are JavaScript and run inside a `with` context of known variables.

```yaml
with:
  cmd: "echo '${event.payload.message}'" # trigger event payload
  cmd: "echo '${event.type}'" # trigger event type
  cmd: "echo '${event.source}'" # source node id
  cmd: "echo '${config.region}'" # workflow config
  cmd: "echo '${secret.API_KEY}'" # from --secrets file
  cmd: "echo '${node.id}'" # this node id
  cmd: "echo '${state.x}'" # persisted state
  cmd: 'cat ${file("/tmp/data.json")}' # read file content at runtime
  cmd: 'echo ${shell("git rev-parse --short HEAD")}' # shell command output
```

Works in any string field inside `with`, recursively.

### Built-in variables

| Variable             | Description                  |
| -------------------- | ---------------------------- |
| `${event}`           | First trigger event          |
| `${event.type}`      | Event type string            |
| `${event.source}`    | Source node id               |
| `${event.payload}`   | Entire payload dict          |
| `${event.payload.x}` | Payload field `x`            |
| `${config.x}`        | Workflow config value        |
| `${secret.x}`        | Secret from `--secrets` file |
| `${node.id}`         | This node's id               |
| `${node.fires}`      | Fire attempt number          |
| `${state.x}`         | Persisted state value        |

### Custom helpers

| Helper  | Syntax                     |
| ------- | -------------------------- |
| `file`  | `${file("/tmp/data.txt")}` |
| `shell` | `${shell("echo hello")}`   |

### `file` helper — read file content at runtime

Reads a file's content and injects it into the template. The file can be
created by an earlier node in the same workflow.

```yaml
- id: review
  driver: shell
  on: [report.generated]
  emits: [review.done]
  with:
    cmd: |
      devin -p "Review this report:
      ${file("/tmp/ac-report.txt")}"
```

### `shell` helper — run a shell command at runtime

Runs a shell command and injects its stdout. Useful for dynamic config values
that are not known until the workflow runs.

```yaml
- id: deploy
  driver: shell
  on: [start]
  emits: [deploy.done]
  with:
    cmd: |
      echo "Deploying ${shell("git rev-parse --short HEAD")}"
```

### Bash compatibility

Shell scripts in `cmd:` blocks often use `${VAR}` and `${VAR:-default}`
for bash variable expansion. Only `${...}` expressions starting with known
prefixes (`event`, `config`, `secret`, `node`, `state`) or the helpers
`file(...)` / `shell(...)` are processed by the template engine. All other
`${...}` expressions are left untouched for the shell to interpret.

## Event matching (wildcards)

| Pattern        | Matches                        |
| -------------- | ------------------------------ |
| `start`        | `start` only                   |
| `lint.*`       | `lint.passed`, `lint.failed`   |
| `task.**`      | `task`, `task.x`, `task.x.y.z` |
| `**.done`      | `done`, `x.done`, `x.y.done`   |
| `shard.*.done` | `shard.42.done`                |

See [Wildcard matching](../reference/wildcard-matching.md) for the full table.

## Handling failures

When a node handler raises an exception, the runner emits two events:

- **`node.failed`** — per-node failure.
- **`workflow.failed`** — workflow-level signal.

Subscribe to `node.failed` to react to failures:

```yaml
- id: alert-on-failure
  on: [node.failed]
  emits: [alert.sent]
  with:
    cmd: "echo 'node ${event.payload.node} failed: ${event.payload.error}'"
```

## Built-in events

| Event                     | When                                          | Source           |
| ------------------------- | --------------------------------------------- | ---------------- |
| `start`                   | Workflow begins                               | `__runner__`     |
| `workflow.done`           | Graph quiesces normally                       | `__runner__`     |
| `workflow.cancel`         | A handler calls `ctx.emit("workflow.cancel")` | `__runner__`     |
| `workflow.cancelled`      | Runner processes cancel                       | `__runner__`     |
| `workflow.failed`         | Any node handler raised an exception          | `__runner__`     |
| `workflow.cycle_detected` | Cycle detection triggered                     | `__runner__`     |
| `node.failed`             | A node handler raised an exception            | failing node id  |
| `event.invalid`           | Payload failed schema validation              | emitting node id |

You can subscribe to any of them with `on:`.

## Node fields reference

| Field             | Default  | Description                                                                     |
| ----------------- | -------- | ------------------------------------------------------------------------------- |
| `id`              | required | Unique node identifier                                                          |
| `driver`          | required | `shell`, `python`, `runner`, `http`, `webhook`, or custom                       |
| `handler`         | `""`     | Named handler (for `python` driver)                                             |
| `on`              | `[]`     | Event types that trigger this node                                              |
| `emits`           | `[]`     | Event types this node produces                                                  |
| `mode`            | `any`    | `any` (first match) or `all` (wait for every pattern)                           |
| `with`            | `{}`     | Config/inputs passed to the handler                                             |
| `max_fires`       | `1`      | Max times this node fires (0 = unlimited, N = bounded)                          |
| `until`           | `[]`     | Stop firing once any of these event types exists                                |
| `retire_on`       | `[]`     | Remove this node from the graph once these events exist                         |
| `retry`           | `{}`     | Retry config. See [Retry](../reference/retry.md).                               |
| `circuit_breaker` | `{}`     | Circuit breaker config. See [Circuit breaker](../reference/circuit-breaker.md). |
| `poll`            | `null`   | Polling sugar. See [Polling](../reference/drivers/polling.md).                  |
| `capabilities`    | `{}`     | Per-node capability grants. See [Safety](../explanation/safety.md).             |

## Workflow-level fields

| Field          | Default | Description                                                                 |
| -------------- | ------- | --------------------------------------------------------------------------- |
| `name`         | `""`    | Workflow name (used for store/DLQ paths)                                    |
| `nodes`        | `[]`    | List of node definitions                                                    |
| `events`       | `[]`    | Optional event schema declarations                                          |
| `config`       | `{}`    | Workflow-level config (accessible via `ctx.config`)                         |
| `include`      | `[]`    | Files to merge                                                              |
| `stop_on`      | `[]`    | Auto-emit `workflow.cancel` when any of these events fire                   |
| `capabilities` | `{}`    | Workflow-level capability defaults. See [Safety](../explanation/safety.md). |

### `stop_on:` — auto-cancel

When any event in `stop_on` fires, the runner auto-emits `workflow.cancel`.
No need for a separate cancel node:

```yaml
name: process-stdin
stop_on: [read.done]
nodes:
  - id: read
    driver: stdin
    on: [start]
    emits: [line.received]
    with:
      event_type: line.received
  - id: process
    driver: shell
    on: [line.received]
    emits: [process.done]
    with:
      cmd: "echo 'got: ${event.payload.line}'"
```
