# Drivers

Drivers execute node logic. A node declares `driver: <name>`, and the runner looks up the driver in its registry. The driver receives the node, trigger events, and the execution context — it runs the handler and returns the events to emit.

## Built-in drivers

- `shell` — runs a shell command.
- `python` — spawns the Python interpreter with `with.script`, `with.module` + `with.function`.
- `runner` — injects new nodes from an event payload into the live graph.
- `http` — makes HTTP requests.
- `webhook` — starts an HTTP server.
- `stdin` — reads lines from stdin.
- `cli` — polls a shell command on an interval.
- `agent` — runs an external AI coding agent CLI.
- `human` — prompts a human on stdin.

## The driver registry

The `DriverRegistry` maps driver names to `Driver` implementations. Built-in drivers are registered by `ac-cli-defaults`. You can also register custom drivers at runtime or through an Effect `Layer`.

```ts
import { Effect } from "effect";
import { DriverRegistry } from "ac-engine";
import { DefaultsLive } from "ac-cli-defaults";

const program = Effect.gen(function* () {
  const registry = yield* DriverRegistry;
  registry.register("echo", EchoDriver);
  // ...
}).pipe(Effect.provide(DefaultsLive));
```

A driver only needs a `name` string and an `invoke` function that returns `Effect<ReadonlyArray<Event>, DriverError>`.

## Agent prefix convention

For `agent:*` driver names, the registry resolves the prefix. Register a driver under `"agent"` and parse the suffix inside `invoke`:

```yaml
- id: review
  driver: agent:claude
  on: [pr.opened]
  emits: [review.done]
  with:
    prompt: "Review this PR: ${event.payload.url}"
```

The `agent:*` scheme lets a single driver support many agent CLIs while the workflow author controls which one runs.

## See also

- [Write a custom driver](../how-to/write-a-custom-driver.md)
- Driver reference: [shell](../reference/drivers/shell.md), [python](../reference/drivers/python.md), [runner](../reference/drivers/runner.md), [http](../reference/drivers/http.md), [webhook](../reference/drivers/webhook.md), [stdin](../reference/drivers/stdin.md), [cli](../reference/drivers/cli.md), [agent](../reference/drivers/agent.md), [human](../reference/drivers/human.md)
- [Streaming](../reference/drivers/streaming.md) and [polling](../reference/drivers/polling.md)
