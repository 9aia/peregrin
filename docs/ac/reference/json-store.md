# JSON store

`JsonStore` from `handl-store-json` is an append-only `EventStore` that persists every event as one line of JSON.

## Programmatic usage

```ts
import { Effect } from "effect";
import { Runner } from "handl-engine";
import { DefaultsLive } from "handl-cli-defaults";
import { Live as JsonStoreLive } from "handl-store-json";

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  return yield* runner.run(workflow);
}).pipe(Effect.provide(DefaultsLive), Effect.provide(JsonStoreLive(".handl/events.jsonl")));

Effect.runPromise(program);
```

## Behaviour

- Each event is one JSON line.
- The file is append-only.
- File permissions are `0600`.
- Events are indexed in memory for fast `byType` and `seenTypes` lookups.

## See also

- [Stores](../explanation/stores.md)
- [Memory store](memory-store.md)
- [Write a custom store](../how-to/write-a-custom-store.md)
