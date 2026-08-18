import { Layer } from "effect";
import { Live as MemoryStoreLive } from "ac-store-memory";
import { Layer as NoopObserverLayer } from "ac-observer-noop";
import { HttpDriver } from "ac-driver-http";
import { NoopDriver } from "ac-driver-noop";
import { PythonDriver } from "ac-driver-python";
import { ShellDriver } from "ac-driver-shell";
import { WebhookDriver } from "ac-driver-webhook";
import { DriverRegistry, DriverRegistryLive, EventBusLiveLayer, RunnerLiveLayer } from "ac-engine";

const registry = new DriverRegistryLive();
registry.register("http", HttpDriver);
registry.register("noop", new NoopDriver());
registry.register("python", PythonDriver);
registry.register("shell", ShellDriver);
registry.register("webhook", WebhookDriver);

const DefaultDriverRegistryLive = Layer.succeed(DriverRegistry, registry);

const base = Layer.merge(MemoryStoreLive, NoopObserverLayer);
const withBus = EventBusLiveLayer.pipe(Layer.provide(base));
const withRegistry = Layer.merge(withBus, DefaultDriverRegistryLive);
const DefaultsLive = RunnerLiveLayer.pipe(Layer.provide(withRegistry));

export { DefaultDriverRegistryLive, withBus, withRegistry };
export { RunnerLiveLayer, EventBusLiveLayer } from "ac-engine";
export { Live as MemoryStoreLive } from "ac-store-memory";
export { Layer as NoopObserverLayer } from "ac-observer-noop";
export { DefaultsLive };
