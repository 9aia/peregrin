# Write a custom driver

A driver is a value that satisfies the `Driver` interface from `ac-core`. It returns an `Effect` that produces the events a node should emit.

## The `Driver` interface

```ts
import { Effect } from "effect";
import { Driver, DriverError, Event } from "ac-core";
import type { ExecutionContext, Node } from "ac-core";

interface Driver {
  readonly name: string;
  readonly invoke: (
    node: Node,
    triggers: ReadonlyArray<Event>,
    ctx: ExecutionContext,
  ) => Effect.Effect<ReadonlyArray<Event>, DriverError, never>;
}
```

- `node` — the node definition (`id`, `driver`, `params`, `emits`, ...).
- `triggers` — the events that caused the node to fire.
- `ctx` — the execution context (config, secrets, event log, `emit`, `inject`, ...).

The driver must return an `Effect` that succeeds with an array of `Event` objects or fails with `DriverError`.

## Example: HTTP driver

```ts
import { Effect } from "effect";
import { Driver, DriverError, Event } from "ac-core";

const HttpDriver: Driver = {
  name: "http",
  invoke: (node, _triggers, _ctx) =>
    Effect.gen(function* () {
      const params = node.params as Record<string, unknown>;
      const method = String(params.method ?? "GET");
      const url = String(params.url);
      const response = yield* Effect.tryPromise({
        try: () => fetch(url, { method }),
        catch: (error) => new DriverError({ reason: String(error) }),
      });
      const body = yield* Effect.tryPromise({
        try: () => response.text(),
        catch: (error) => new DriverError({ reason: String(error) }),
      });
      const status = response.status;
      const payload = { status, body };
      const eventType = status < 400 ? node.emits[0] : node.emits[1];
      return [new Event(eventType, payload, node.id)];
    }),
};
```

Use in a workflow:

```yaml
- id: webhook
  driver: http
  on: [deploy.ready]
  emits: [webhook.sent, webhook.failed]
  params:
    method: POST
    url: "https://api.slack.com/services/notify"
    body:
      text: "Deployment ready"
```

## Register the driver

Register the driver through the `DriverRegistry` at runtime, or build an Effect `Layer` that provides it:

```ts
import { Effect } from "effect";
import { Driver, DriverError, Event } from "ac-core";
import {
  DriverRegistry,
  DriverRegistryLiveLayer,
  Runner,
  RunnerLiveLayer,
  EventBusLiveLayer,
} from "ac-engine";
import { DefaultsLive } from "ac-cli-defaults";

const echoDriver: Driver = {
  name: "echo",
  invoke: (node, _triggers, _ctx) =>
    Effect.succeed([new Event(`${node.id}.done`, { message: node.params.message }, node.id)]),
};

const program = Effect.gen(function* () {
  const registry = yield* DriverRegistry;
  registry.register("echo", echoDriver);
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  console.log(`emitted ${log.length} events`);
}).pipe(Effect.provide(DefaultsLive));

Effect.runPromise(program);
```

## Agent prefix convention

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

- [Drivers](../explanation/drivers.md) — the driver model
- [Custom drivers and handlers](custom-drivers.md) — more driver recipes
- [Execution context](../reference/handler-context.md) — the `ctx` object
