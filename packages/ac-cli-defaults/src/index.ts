import { Layer } from "effect";
import { Live as MemoryStoreLive } from "ac-store-memory";
import { Layer as NoopObserverLayer } from "ac-observer-noop";
import { AgentDriver } from "ac-driver-agent";
import { CliDriver } from "ac-driver-cli";
import { HttpDriver } from "ac-driver-http";
import { HumanDriver } from "ac-driver-human";
import { NoopDriver } from "ac-driver-noop";
import { PollingDriver } from "ac-driver-polling";
import { PythonDriver } from "ac-driver-python";
import { RunnerDriver } from "ac-driver-runner";
import { ShellDriver } from "ac-driver-shell";
import { StdinDriver } from "ac-driver-stdin";
import { StreamingDriver } from "ac-driver-streaming";
import { WebhookDriver } from "ac-driver-webhook";
import { DriverRegistry, DriverRegistryLive, EventBusLiveLayer, RunnerLiveLayer } from "ac-engine";

const registry = new DriverRegistryLive();
registry.register("agent", AgentDriver);
registry.register("cli", CliDriver);
registry.register("http", HttpDriver);
registry.register("human", HumanDriver);
registry.register("noop", new NoopDriver());
registry.register("polling", PollingDriver);
registry.register("python", PythonDriver);
registry.register("runner", RunnerDriver);
registry.register("shell", ShellDriver);
registry.register("stdin", StdinDriver);
registry.register("streaming", StreamingDriver);
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
