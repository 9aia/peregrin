# Execution context

The `ExecutionContext` object gives drivers access to workflow config, secrets, the event log, and runtime actions.

```ts
import { Event } from "handl-core";

const exampleDriver = {
  name: "example",
  invoke: (node, _triggers, ctx) =>
    Effect.gen(function* () {
      // Read workflow config
      const region = String(ctx.config.region ?? "us-east-1");

      // Read secrets — preferred API
      const apiKey = ctx.getSecret("API_KEY");

      // Direct access also works
      const sameKey = ctx.secrets.API_KEY;

      // Idempotency key — unique per node firing
      const key = ctx.idempotencyKey;

      // Read the full event log
      for (const event of ctx.log) {
        console.log(event.type, event.payload);
      }

      // Emit events dynamically
      ctx.emit(new Event("dynamic.event", { data: 42 }, node.id));

      // Inject new nodes at runtime
      ctx.inject([
        {
          id: "dynamic-1",
          driver: "shell",
          on: ["start"],
          emits: ["dynamic-1.done"],
          params: { cmd: "echo hi" },
        },
      ]);

      // Fire counter for this node
      const attempt = ctx.fires;

      // Shared scratch state
      ctx.state.set("last_run", Date.now());

      // Request shutdown from inside a driver
      ctx.requestShutdown();

      return [new Event("done", {}, node.id)];
    }),
};
```

## Attributes and methods

| Name                            | Description                            |
| ------------------------------- | -------------------------------------- |
| `ctx.config`                    | Workflow-level `config` record         |
| `ctx.secrets`                   | Record of resolved secrets             |
| `ctx.getSecret(name, default?)` | Returns a secret or the default        |
| `ctx.log`                       | All events emitted so far              |
| `ctx.idempotencyKey`            | Unique string per node firing          |
| `ctx.fires`                     | 1-based attempt counter for this node  |
| `ctx.state`                     | `StateStore` for scratch state         |
| `ctx.emit(event)`               | Emit an `Event` directly               |
| `ctx.inject(nodeSpecs)`         | Inject new nodes into the graph        |
| `ctx.dryRun`                    | Whether the runner is in dry-run mode  |
| `ctx.strict`                    | Whether the workflow is in strict mode |
| `ctx.traceparent`               | W3C traceparent string for propagation |
| `ctx.requestShutdown()`         | Cancel the workflow at the next cycle  |

## See also

- [Drivers](../explanation/drivers.md) — the driver model
- [Dynamically inject nodes](../how-to/dynamically-inject-nodes.md)
- [Register a custom driver](handler-registration.md)
