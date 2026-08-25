# Use the programmatic API

Everything the CLI can do, you can do from TypeScript using the `handl-core` model, the `handl-engine` runner, and the `handl-loader-yaml` loader. The runtime is built on [Effect](https://effect.website), so program setup and dependency wiring use `Layer` and `Effect.runPromise`.

## Basic run

```ts
import { Effect } from "effect";
import { Node, Workflow } from "handl-core";
import { Runner, RunnerLiveLayer } from "handl-engine";
import { DefaultsLive } from "handl-cli-defaults";

const workflow = new Workflow(
  "hello",
  [],
  [new Node("hello", "shell", "", ["start"], ["hello.done"], "any", { cmd: "echo 'hello'" })],
);

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  for (const event of log) {
    console.log(`  ${event.type} [${event.source}]`);
  }
}).pipe(Effect.provide(DefaultsLive));

Effect.runPromise(program);
```

`DefaultsLive` wires the default drivers, an in-memory `EventStore`, and the no-op observer.

## Load from YAML

```ts
import { Effect } from "effect";
import { loadWorkflow } from "handl-loader-yaml";
import { Runner } from "handl-engine";
import { DefaultsLive } from "handl-cli-defaults";

const program = Effect.gen(function* () {
  const workflow = yield* loadWorkflow("workflow.yaml");
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  console.log(`run complete: ${log.length} events`);
}).pipe(Effect.provide(DefaultsLive));

Effect.runPromise(program);
```

## Choose a store

The default CLI layer uses an in-memory `MemoryStore`. For a persistent JSON-lines store, provide `JsonStore` instead:

```ts
import { Live as JsonStoreLive } from "handl-store-json";
import { Live as MemoryStoreLive } from "handl-store-memory";

const program = ...
  .pipe(Effect.provide(DefaultsLive))
  .pipe(Effect.provide(JsonStoreLive(".handl/events.jsonl")));
```

The `JsonStore` layer overrides the in-memory store because both satisfy the `EventStore` adapter port.

## Inspect the event log

The runner returns the final log as an array of `Event` objects:

```ts
const log = yield * runner.run(workflow);

for (const event of log) {
  console.log(event.type, event.payload, event.source);
}

// Query by type
const passed = log.filter((e) => e.type === "lint.passed");
```

For direct store access, implement or yield the `EventStore` port:

```ts
import { EventStore } from "handl-core";

const store = yield * EventStore;
const all = store.all();
const byType = store.byType("lint.passed");
```

## Add observability

Provide the `OtelObserver` layer to emit console span output:

```ts
import { OtelObserverLive } from "handl-observer-otel";

const program = ...
  .pipe(Effect.provide(DefaultsLive))
  .pipe(Effect.provide(OtelObserverLive));
```

## Pass secrets

```ts
const program = runner.run(workflow, { secrets: { API_KEY: "secret" } });
```

Secrets are resolved through `ctx.getSecret` in drivers and the template engine.

## Dry run

```ts
const log = yield * runner.run(workflow, { dryRun: true });
```

In dry-run mode, drivers return a synthetic event and do not execute side effects.

## See also

- [Write a custom driver](write-a-custom-driver.md) — implement a `Driver`
- [Write a custom store](write-a-custom-store.md) — implement `EventStore`
- [Write a custom observer](write-a-custom-observer.md) — implement `Observer`
- [Run a workflow](../tutorials/run-a-workflow.md) — CLI usage
