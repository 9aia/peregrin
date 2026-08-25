import http from "node:http";
import { Layer } from "effect";
import {
  Observer,
  type Span,
  type Workflow,
  type Node,
  type Event,
  type StateStore,
} from "handl-core";
import { EventStoreLive, StateStoreLive } from "handl-store-memory";
import { NoopSpan, Layer as NoopObserverLayer } from "handl-observer-noop";
import { Live as LiteLLMLive, LiteLLMDriver } from "handl-plugin-litellm";
import { Live as AppriseLive, AppriseDriver } from "handl-plugin-apprise";
import { AgentDriver } from "handl-driver-agent";
import { CliDriver } from "handl-driver-cli";
import { HttpDriver } from "handl-driver-http";
import { HumanDriver } from "handl-driver-human";
import { NoopDriver } from "handl-driver-noop";
import { PollingDriver } from "handl-driver-polling";
import { PythonDriver } from "handl-driver-python";
import { RunnerDriver } from "handl-driver-runner";
import { ShellDriver } from "handl-driver-shell";
import { StdinDriver } from "handl-driver-stdin";
import { StreamingDriver } from "handl-driver-streaming";
import { WebhookDriver } from "handl-driver-webhook";
import {
  DriverRegistry,
  DriverRegistryLive,
  EventBusLiveLayer,
  RunnerLiveLayer,
} from "handl-engine";

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

export const makeDefaultsLive = (
  observer: Layer.Layer<Observer>,
  stateStore: Layer.Layer<StateStore> = StateStoreLive,
) => {
  const base = Layer.merge(Layer.merge(EventStoreLive, observer), plugins);
  const withBus = EventBusLiveLayer.pipe(Layer.provide(base));
  const withRegistry = Layer.merge(
    Layer.merge(Layer.merge(withBus, DefaultDriverRegistryLive), observer),
    stateStore,
  );
  return RunnerLiveLayer.pipe(Layer.provide(withRegistry));
};

export interface MetricsObserverConfig {
  readonly port?: number;
  readonly host?: string;
  readonly logFormat?: string;
}

export class MetricsObserver implements Observer {
  private eventCounts = new Map<string, number>();
  private nodeFires = new Map<string, number>();
  private circuitTrips = 0;
  private dlq = 0;
  private workflowStatus = 0;
  private port = 0;
  private host = "127.0.0.1";
  private logFormat = "text";
  private server: http.Server | null = null;

  constructor(config: MetricsObserverConfig = {}) {
    this.port = config.port ?? 0;
    this.host = config.host ?? "127.0.0.1";
    this.logFormat = config.logFormat ?? "text";
  }

  private renderMetrics(): string {
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
    lines.push(`ac_workflow_status ${this.workflowStatus}`);
    return lines.join("\n") + "\n";
  }

  private emitMetrics(): void {
    if (this.port > 0) return;
    if (this.logFormat === "json") {
      const events: Record<string, number> = {};
      for (const [type, count] of this.eventCounts) events[type] = count;
      const nodes: Record<string, number> = {};
      for (const [id, count] of this.nodeFires) nodes[id] = count;
      console.log(
        JSON.stringify({
          ts: new Date().toISOString(),
          source: "metrics",
          events,
          nodes,
          circuit_trips: this.circuitTrips,
          dlq: this.dlq,
          workflow_status: this.workflowStatus,
        }),
      );
      return;
    }
    console.log(this.renderMetrics());
  }

  private handleRequest(req: http.IncomingMessage, res: http.ServerResponse): void {
    if (req.url === "/metrics") {
      res.writeHead(200, { "Content-Type": "text/plain; version=0.0.4" });
      res.end(this.renderMetrics());
      return;
    }
    if (req.url === "/health") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ status: "ok", events: this.eventCounts.size }));
      return;
    }
    res.writeHead(404);
    res.end("Not found\n");
  }

  onWorkflowStart(_workflow: Workflow): void {
    this.eventCounts.clear();
    this.nodeFires.clear();
    this.circuitTrips = 0;
    this.dlq = 0;
    this.workflowStatus = 0;
    if (this.port <= 0) return;
    this.server = http.createServer((req, res) => this.handleRequest(req, res));
    this.server.on("error", (err) => {
      console.error(`[metrics] server error: ${err.message}`);
    });
    try {
      this.server.listen(this.port, this.host, () => {
        console.log(`[metrics] listening on http://${this.host}:${this.port}`);
      });
    } catch (err) {
      console.error(`[metrics] failed to start: ${String(err)}`);
    }
  }

  onWorkflowEnd(_eventCount: number, cancelled: boolean, _cycle: boolean, failed: boolean): void {
    const status = cancelled || failed ? -1 : 1;
    this.workflowStatus = status;
    this.emitMetrics();
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
export { RunnerLiveLayer, EventBusLiveLayer } from "handl-engine";
export { Live as MemoryStoreLive } from "handl-store-memory";
export { Layer as NoopObserverLayer } from "handl-observer-noop";
export { OtelObserver, OtelObserverLive } from "handl-observer-otel";
export { DefaultsLive };
