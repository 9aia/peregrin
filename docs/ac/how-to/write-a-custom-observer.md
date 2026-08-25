# Write a custom observer

Implement the `Observer` interface from `handl-core` and provide it as an Effect `Layer`.

## The `Observer` and `Span` interfaces

```ts
import { Observer, Span, Event, Workflow, Node } from "handl-core";

const MyObserver: Observer = {
  onWorkflowStart(workflow: Workflow) {
    console.log(`START ${workflow.name}`);
  },
  onWorkflowEnd(eventCount, cancelled, cycle, failed) {
    console.log(`END events=${eventCount} cancelled=${cancelled} cycle=${cycle} failed=${failed}`);
  },
  onNodeStart(node, _triggers): Span {
    return {
      end: () => {},
      setAttribute: () => {},
      recordError: () => {},
      getTraceparent: () => "",
    };
  },
  onNodeEnd(_node, _events, _error, span) {
    span.end();
  },
  onEvent(event: Event) {
    console.log(`  EVENT ${event.type} [${event.source}]`);
  },
  onInject(_nodeSpecs) {
    console.log(`  INJECTED`);
  },
  onRetire(nodeId) {
    console.log(`  RETIRED ${nodeId}`);
  },
};
```

## Provide the observer

```ts
import { Layer, Effect } from "effect";
import { Observer } from "handl-core";
import { Runner } from "handl-engine";
import { DefaultsLive } from "handl-cli-defaults";

const MyObserverLive = Layer.succeed(Observer, MyObserver);

const program = Effect.gen(function* () {
  const runner = yield* Runner;
  const log = yield* runner.run(workflow);
  return log;
}).pipe(Effect.provide(DefaultsLive), Effect.provide(MyObserverLive));

Effect.runPromise(program);
```

`MyObserverLive` satisfies the `Observer` port and overrides the default no-op observer.

## Logging observer

```ts
const LoggingObserver: Observer = {
  onWorkflowStart(workflow) {
    console.log(`START workflow='${workflow.name}' nodes=${workflow.nodes.length}`);
  },
  onWorkflowEnd(eventCount, cancelled, cycle, failed) {
    const status = failed ? "FAILED" : cancelled ? "CANCELLED" : cycle ? "CYCLE" : "DONE";
    console.log(`END status=${status} events=${eventCount}`);
  },
  onNodeStart(node, _triggers) {
    console.log(`  NODE ${node.id} (driver=${node.driver})`);
    return { end() {}, setAttribute() {}, recordError() {}, getTraceparent: () => "" };
  },
  onNodeEnd(node, events, error, span) {
    if (error) console.error(`  ERROR ${node.id}: ${error.message}`);
    else console.log(`  DONE ${node.id} → ${events.length} events`);
    span.end();
  },
  onEvent(event) {
    console.log(`  EVENT ${event.type} [${event.source}]`);
  },
  onInject(_nodeSpecs) {
    console.log(`  INJECTED`);
  },
  onRetire(nodeId) {
    console.log(`  RETIRED ${nodeId}`);
  },
};
```

## Datadog-style metrics

Emitting metrics inside the hooks is the same as in any TypeScript service:

```ts
import { StatsD } from "node-statsd";

const client = new StatsD();

const DatadogObserver: Observer = {
  onWorkflowStart(workflow) {
    client.increment(`workflow.start,workflow:${workflow.name}`);
  },
  onWorkflowEnd(eventCount, _cancelled, _cycle, failed) {
    client.histogram("workflow.events", eventCount);
    if (failed) client.increment("workflow.failure");
  },
  onNodeStart(_node, _triggers) {
    return { end() {}, setAttribute() {}, recordError() {}, getTraceparent: () => "" };
  },
  onNodeEnd(node, _events, error, span) {
    if (error) client.increment(`node.failure,node:${node.id}`);
    span.end();
  },
  onEvent(_event) {},
  onInject(_nodeSpecs) {},
  onRetire(_nodeId) {},
};
```

## See also

- [Observers](../explanation/observers.md) — the adapter concept
- [OpenTelemetry observer](../reference/otel-observer.md)
- [Use the programmatic API](programmatic-api.md)
