# Register a custom driver

Custom drivers are registered by name in the `DriverRegistry`.

## Register at runtime

```ts
import { Effect } from "effect";
import { Driver } from "ac-core";
import { DriverRegistry, Runner } from "ac-engine";
import { DefaultsLive } from "ac-cli-defaults";

const EchoDriver: Driver = {
  name: "echo",
  invoke: (node, _triggers, _ctx) =>
    Effect.succeed([new Event(`${node.id}.done`, { text: node.with_.text }, node.id)]),
};

const program = Effect.gen(function* () {
  const registry = yield* DriverRegistry;
  registry.register("echo", EchoDriver);
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  return log;
}).pipe(Effect.provide(DefaultsLive));
```

## Use in a workflow

```yaml
- id: custom
  driver: echo
  on: [start]
  emits: [custom.done]
  with:
    text: "hello"
```

## Prefix convention

The registry resolves `driver` names by prefix. Registering a driver under `"agent"` makes it handle `agent:claude`, `agent:cursor`, and so on. The suffix is available on `node.driver` for the `invoke` implementation to parse.

## See also

- [Write a custom driver](../how-to/write-a-custom-driver.md)
- [Drivers](../explanation/drivers.md) — the driver model
- [Execution context](handler-context.md)
