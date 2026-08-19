import { Layer } from "effect";
import { Observer, type Span, type Workflow, type Node, type Event } from "ac-core";
import { Live as MemoryStoreLive } from "ac-store-memory";
import { NoopSpan, Layer as NoopObserverLayer } from "ac-observer-noop";
import { Live as LiteLLMLive, LiteLLMDriver } from "ac-plugin-litellm";
import { Live as AppriseLive, AppriseDriver } from "ac-plugin-apprise";
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
registry.register("litellm", LiteLLMDriver);
registry.register("apprise", AppriseDriver);

const DefaultDriverRegistryLive = Layer.succeed(DriverRegistry, registry);

const plugins = Layer.merge(LiteLLMLive, AppriseLive);

export const makeDefaultsLive = (observer: Layer.Layer<Observer>) => {
  const base = Layer.merge(Layer.merge(MemoryStoreLive, observer), plugins);
  const withBus = EventBusLiveLayer.pipe(Layer.provide(base));
  const withRegistry = Layer.merge(Layer.merge(withBus, DefaultDriverRegistryLive), observer);
  return RunnerLiveLayer.pipe(Layer.provide(withRegistry));
};

export class MetricsObserver implements Observer {
  private eventCounts = new Map<string, number>();
  private nodeFires = new Map<string, number>();
  private circuitTrips = 0;
  private dlq = 0;

  onWorkflowStart(_workflow: Workflow): void {
    this.eventCounts.clear();
    this.nodeFires.clear();
    this.circuitTrips = 0;
    this.dlq = 0;
  }

  onWorkflowEnd(_eventCount: number, _cancelled: boolean, _cycle: boolean, failed: boolean): void {
    const status = failed ? -1 : 0;
    const lines: Array<string> = [];
    lines.push("# HELP ac_events_total Total events emitted by type");
    lines.push("# TYPE ac_events_total counter");
    for (const [eventType, count] of [...this.eventCounts.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    )) {
      lines.push(`ac_events_total{type="${eventType}"} ${count}`);
    }
    lines.push("# HELP ac_node_fires_total Total node firings by node id");
    lines.push("# TYPE ac_node_fires_total counter");
    for (const [nodeId, count] of [...this.nodeFires.entries()].sort((a, b) =>
      a[0].localeCompare(b[0]),
    )) {
      lines.push(`ac_node_fires_total{node="${nodeId}"} ${count}`);
    }
    lines.push("# HELP ac_circuit_trips_total Total circuit breaker trips");
    lines.push("# TYPE ac_circuit_trips_total counter");
    lines.push(`ac_circuit_trips_total ${this.circuitTrips}`);
    lines.push("# HELP ac_dlq_depth Current dead-letter queue depth");
    lines.push("# TYPE ac_dlq_depth gauge");
    lines.push(`ac_dlq_depth ${this.dlq}`);
    lines.push(
      "# HELP ac_workflow_status Workflow status (1=done, 0=running, -1=failed/cancelled)",
    );
    lines.push("# TYPE ac_workflow_status gauge");
    lines.push(`ac_workflow_status ${status}`);
    console.log(lines.join("\n"));
  }

  onNodeStart(node: Node, _triggers: ReadonlyArray<Event>): Span {
    this.nodeFires.set(node.id, (this.nodeFires.get(node.id) ?? 0) + 1);
    return new NoopSpan();
  }

  onNodeEnd(_node: Node, _events: ReadonlyArray<Event>, _error: Error | null, span: Span): void {
    span.end();
  }

  onEvent(event: Event): void {
    this.eventCounts.set(event.type, (this.eventCounts.get(event.type) ?? 0) + 1);
    if (event.type === "circuit.open") {
      this.circuitTrips += 1;
    }
    if (event.type === "dlq.event") {
      this.dlq += 1;
    }
  }

  onInject(_nodeSpecs: ReadonlyArray<Record<string, unknown>>): void {}

  onRetire(_nodeId: string): void {}
}

export const MetricsObserverLive = Layer.succeed(Observer, new MetricsObserver());

const DefaultsLive = makeDefaultsLive(NoopObserverLayer);

export { DefaultDriverRegistryLive };
export { RunnerLiveLayer, EventBusLiveLayer } from "ac-engine";
export { Live as MemoryStoreLive } from "ac-store-memory";
export { Layer as NoopObserverLayer } from "ac-observer-noop";
export { OtelObserverLive } from "ac-observer-otel";
export { DefaultsLive };
