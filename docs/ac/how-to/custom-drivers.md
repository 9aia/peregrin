# Custom drivers and handlers

Peregrin drivers are implemented in TypeScript. The `python` driver is a shell-out to the Python interpreter, not a Python SDK.

## Shell driver conventions

The `shell` driver is the universal tool — most CLI agents are just shell commands:

```yaml
- id: review
  driver: shell
  on: [pr.opened]
  emits: [review.done, review.failed]
  params:
    cmd: "claude --print 'Review this PR: ${event.payload.url}'"
```

### Exit codes

The `shell` driver emits `cmd.done` on exit code `0` and `node.failed` on any non-zero exit code.

```yaml
- id: deploy
  driver: shell
  on: [test.passed]
  emits: [deploy.done, deploy.failed]
  params:
    cmd: "deploy.sh"
    timeout: 300
```

- `params.cmd` — the command to run.
- `params.timeout` — timeout in milliseconds (default `30000`).

## The `python` driver

The `python` driver spawns the `python` interpreter. It is not a Python SDK.

```yaml
- id: analyse
  driver: python
  on: [data.ready]
  emits: [analyse.done, analyse.failed]
  params:
    script: scripts/analyse.py
    args: ["--input", "${event.payload.path}"]
    timeout: 60000
```

### `params` fields

| Field      | Type   | Description                             |
| ---------- | ------ | --------------------------------------- |
| `script`   | string | Path to a Python script to execute      |
| `module`   | string | Module to import (requires `function`)  |
| `function` | string | Function to call (requires `module`)    |
| `args`     | array  | Command-line arguments for `script`     |
| `timeout`  | number | Timeout in milliseconds (default 30000) |

Use `params.module` and `params.function` to call a single function:

```yaml
- id: compute
  driver: python
  on: [start]
  emits: [compute.done, compute.failed]
  params:
    module: myapp.math
    function: compute_total
```

The driver emits `python.done` (rc `0`) or `python.failed` (non-zero or runtime error).

## Custom TypeScript drivers

Implement the `Driver` interface and register it with `DriverRegistry`:

```ts
import { Effect } from "effect";
import { Driver, Event } from "ac-core";

const GreeterDriver: Driver = {
  name: "greeter",
  invoke: (node, _triggers, _ctx) =>
    Effect.succeed([new Event(`${node.id}.done`, { text: `Hello, ${node.params.name}` }, node.id)]),
};
```

Register at runtime:

```ts
import { Effect } from "effect";
import { DriverRegistry } from "ac-engine";
import { DefaultsLive } from "ac-cli-defaults";

const program = Effect.gen(function* () {
  const registry = yield* DriverRegistry;
  registry.register("greeter", GreeterDriver);
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  console.log(log);
}).pipe(Effect.provide(DefaultsLive));

Effect.runPromise(program);
```

## Agent drivers

Most CLI agents (claude-code, cursor, devin) are just shell commands. Use the `shell` driver directly, or register a thin custom driver for richer APIs.

For `agent:*` driver names, the registry resolves the prefix. Register a driver under `"agent"` and parse the suffix inside `invoke`:

```yaml
- id: review
  driver: agent:claude
  on: [pr.opened]
  emits: [review.done]
  params:
    prompt: "Review this PR: ${event.payload.url}"
```

## See also

- [Write a custom driver](write-a-custom-driver.md) — implement a `Driver`
- [Drivers](../explanation/drivers.md) — the driver model
- [python driver](../reference/drivers/python.md) — reference
- [Use the programmatic API](programmatic-api.md)
