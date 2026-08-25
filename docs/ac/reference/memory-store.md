# Memory store

`MemoryStore` from `handl-store-memory` is an in-memory `EventStore`. Fast, no persistence.

## Programmatic usage

```ts
import { Effect } from "effect";
import { Runner } from "handl-engine";
import { DefaultsLive } from "handl-cli-defaults";
import { Live as MemoryStoreLive } from "handl-store-memory";

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  return yield* runner.run(workflow);
}).pipe(Effect.provide(DefaultsLive), Effect.provide(MemoryStoreLive));
```

## When to use

- Tests that do not need crash recovery.
- Ephemeral runs where you do not want to leave state on disk.
- Benchmarks that want to exclude disk I/O.

`DefaultsLive` already includes `MemoryStoreLive`, so no extra layer is needed for the default in-memory behaviour.

## See also

- [Stores](../explanation/stores.md)
- [JSON store](json-store.md)
