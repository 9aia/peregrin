import { Effect, Layer } from "effect";
import { Driver } from "handl-core";
import type { DriverError, Event, ExecutionContext, Node, Workflow } from "handl-core";
import {
  DriverRegistry,
  DriverRegistryLiveLayer,
  EventBusLiveLayer,
  Runner,
  RunnerLiveLayer,
} from "handl-engine";
import { Live as MemoryStoreLayer } from "handl-store-memory";
import { Layer as NoopObserverLayer } from "handl-observer-noop";

export class FakeDriver implements Driver {
  constructor(
    readonly name: string,
    readonly events: ReadonlyArray<Event>,
  ) {}

  invoke(
    _node: Node,
    _triggers: ReadonlyArray<Event>,
    _ctx: ExecutionContext,
  ): Effect.Effect<ReadonlyArray<Event>, DriverError, never> {
    return Effect.succeed(this.events);
  }
}

export const fakeDriverLayer = (
  name: string,
  events: ReadonlyArray<Event>,
): Layer.Layer<Driver, never, DriverRegistry> =>
  Layer.effect(
    Driver,
    Effect.gen(function* () {
      const registry = yield* DriverRegistry;
      const driver = new FakeDriver(name, events);
      registry.register(name, driver);
      return driver;
    }),
  );

export const FakeDriverLayer = fakeDriverLayer;

export const runWorkflow = (
  workflow: Workflow,
  drivers: ReadonlyArray<Driver> = [],
): Effect.Effect<ReadonlyArray<Event>, DriverError, never> =>
  Effect.gen(function* () {
    const registry = yield* DriverRegistry;
    yield* Effect.sync(() => {
      for (const driver of drivers) {
        registry.register(driver.name, driver);
      }
    });
    const runner = yield* Runner;
    const log = yield* runner.run(workflow);
    return log;
  }).pipe(
    Effect.provide(RunnerLiveLayer),
    Effect.provide(EventBusLiveLayer),
    Effect.provide(DriverRegistryLiveLayer),
    Effect.provide(NoopObserverLayer),
    Effect.provide(MemoryStoreLayer),
  );
