# Adapter ports

The runner has two adapter ports — swappable implementations for persistence and observability. The runner does not know or care which implementation you use.

## EventStore (persistence)

The event log flows through an `EventStore`. The bus appends events; the runner reads them. Swap implementations without touching the bus or runner.

```ts
interface EventStore {
  append(event: Event): void;
  all(): ReadonlyArray<Event>;
  byType(eventType: string): ReadonlyArray<Event>;
  seenTypes(): ReadonlySet<string>;
  latest(): Event | null;
  clear(): void;
  close(): void;
}
```

### Implementations

- `JsonStore` (from `handl-store-json`) — append-only JSON lines file.
- `MemoryStore` (from `handl-store-memory`) — in-memory, fast, no persistence.
- Custom — implement the `EventStore` interface and provide it as a `Layer`.

### Why an adapter port?

The event log is the workflow state. Different deployments need different storage: a file for single-process production, a database for distributed workflows, in-memory for tests. The adapter port lets you swap without touching the core.

The store also provides `seenTypes()` — a fast lookup for which event types have been seen. The bus uses this for `mode: all` eligibility checks instead of scanning the full log.

### Writing a custom store

```ts
import { Effect, Layer } from "effect";
import { Event, EventStore } from "handl-core";

class MyRedisStore implements EventStore {
  // ... implement all, byType, seenTypes, latest, clear, close
}

const MyRedisLive = Layer.succeed(EventStore, new MyRedisStore({/* ... */}));

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  return yield* runner.run(workflow);
}).pipe(Effect.provide(DefaultsLive), Effect.provide(MyRedisLive));
```

## Observer (observability)

Lifecycle hooks flow through an `Observer`. The runner calls the observer at key points: workflow start/end, node start/end, event emission, node injection, and node retirement.

```ts
interface Observer {
  onWorkflowStart(workflow: Workflow): void;
  onWorkflowEnd(eventCount: number, cancelled: boolean, cycle: boolean, failed: boolean): void;
  onNodeStart(node: Node, triggers: ReadonlyArray<Event>): Span;
  onNodeEnd(node: Node, events: ReadonlyArray<Event>, error: Error | null, span: Span): void;
  onEvent(event: Event): void;
  onInject(nodeSpecs: ReadonlyArray<Record<string, unknown>>): void;
  onRetire(nodeId: string): void;
}
```

### Implementations

| Observer                 | Use                                |
| ------------------------ | ---------------------------------- |
| `NoopObserver` (default) | Silent, zero overhead              |
| `OtelObserver`           | Console span output                |
| Custom                   | Implement the `Observer` interface |

### Why an adapter port?

Observability is deployment-specific. A local dev run does not need tracing. A production run might use OTel, Datadog, or a logging observer. The adapter port lets you swap without touching the runner.

The `Span` returned by `onNodeStart` is ended by `onNodeEnd`. This decouples span lifecycle from the runner — the observer decides whether to create a real trace span or a no-op.

```ts
const LoggingObserver: Observer = {
  onWorkflowStart(workflow) {
    console.log(`START ${workflow.name}`);
  },
  onWorkflowEnd(eventCount) {
    console.log(`END ${eventCount}`);
  },
  onNodeStart(node) {
    console.log(`  NODE ${node.id}`);
    return { end() {}, setAttribute() {}, recordError() {}, getTraceparent: () => "" };
  },
  onNodeEnd(_node, _events, _error, span) {
    span.end();
  },
  onEvent(_event) {},
  onInject(_nodeSpecs) {},
  onRetire(_nodeId) {},
};
```
