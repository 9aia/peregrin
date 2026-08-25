# OpenTelemetry observer

`OtelObserver` from `handl-observer-otel` writes console span output for the workflow and every node.

## Usage

```ts
import { Effect } from "effect";
import { Runner } from "handl-engine";
import { DefaultsLive } from "handl-cli-defaults";
import { OtelObserverLive } from "handl-observer-otel";

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  return log;
}).pipe(Effect.provide(DefaultsLive), Effect.provide(OtelObserverLive));

Effect.runPromise(program);
```

## Span output

```
[otel:workflow:start] my-workflow
[otel:node:start] hello traceparent=00-...
[otel:event] hello.done
[otel:node:end] hello emitted=1
[otel:workflow:end] workflow:my-workflow events=2 cancelled=false cycle=false failed=false
```

Each span receives a generated W3C `traceparent`. The `http` driver can read `ctx.traceparent` and propagate it in outbound requests.

## See also

- [Observers](../explanation/observers.md)
- [Write a custom observer](../how-to/write-a-custom-observer.md)
