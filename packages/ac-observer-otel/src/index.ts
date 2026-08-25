import crypto from "node:crypto";
import { Layer, Option } from "effect";
import { Observer, type Span, type Workflow, type Node, type Event } from "handl-core";

function generateTraceparent(): string {
  const traceId = crypto.randomBytes(16).toString("hex");
  const spanId = crypto.randomBytes(8).toString("hex");
  return `00-${traceId}-${spanId}-01`;
}

export interface OtelObserverConfig {
  readonly endpoint?: string;
  readonly protocol?: string;
  readonly logFormat?: string;
  readonly serviceName?: string;
}

class OtelSpan implements Span {
  private isEnded = false;
  private attributes: Record<string, unknown> = {};

  readonly traceparent: string;

  constructor(readonly name: string) {
    this.traceparent = generateTraceparent();
  }

  end(): void {
    if (this.isEnded) return;
    this.isEnded = true;
  }

  setAttribute(key: string, value: unknown): void {
    this.attributes[key] = value;
  }

  recordError(error: Error): void {
    this.setAttribute("error", `${error.name}: ${error.message}`);
  }

  getTraceparent(): string {
    return this.traceparent;
  }
}

export class OtelObserver implements Observer {
  readonly spans: Array<OtelSpan> = [];
  readonly events: Array<Event> = [];
  readonly retirements: Array<string> = [];
  private workflowSpan = Option.none<OtelSpan>();
  private endpoint?: string;
  private protocol = "grpc";
  private logFormat = "text";
  private serviceName = "agent-campaign";

  constructor(config: OtelObserverConfig = {}) {
    this.endpoint = config.endpoint;
    this.protocol = config.protocol ?? "grpc";
    this.logFormat = config.logFormat ?? "text";
    this.serviceName = config.serviceName ?? "agent-campaign";
  }

  private emit(level: string, message: string, extra: Record<string, unknown> = {}): void {
    if (this.logFormat === "json") {
      console.log(
        JSON.stringify({
          ts: new Date().toISOString(),
          level,
          source: "otel",
          service: this.serviceName,
          message,
          ...extra,
        }),
      );
      return;
    }
    const suffix = Object.keys(extra).length > 0 ? ` ${JSON.stringify(extra)}` : "";
    console.log(`[otel:${level}] ${message}${suffix}`);
  }

  onWorkflowStart(workflow: Workflow): void {
    const span = new OtelSpan(`workflow:${workflow.name}`);
    span.setAttribute("workflow.name", workflow.name);
    span.setAttribute("workflow.nodes", workflow.nodes.length);
    this.spans.push(span);
    this.workflowSpan = Option.some(span);
    const extra: Record<string, unknown> = { nodes: workflow.nodes.length };
    if (this.endpoint) {
      extra.endpoint = this.endpoint;
      extra.protocol = this.protocol;
    }
    this.emit("workflow:start", workflow.name, extra);
  }

  onWorkflowEnd(eventCount: number, cancelled: boolean, cycle: boolean, failed: boolean): void {
    Option.match(this.workflowSpan, {
      onNone: () => void 0,
      onSome: (span) => {
        span.setAttribute("workflow.events", eventCount);
        span.setAttribute("workflow.cancelled", cancelled);
        span.setAttribute("workflow.cycle", cycle);
        span.setAttribute("workflow.failed", failed);
        span.end();
        this.workflowSpan = Option.none();
        this.emit("workflow:end", span.name, {
          events: eventCount,
          cancelled,
          cycle,
          failed,
        });
      },
    });
  }

  onNodeStart(node: Node, triggers: ReadonlyArray<Event>): Span {
    const span = new OtelSpan(`node:${node.id}`);
    span.setAttribute("node.id", node.id);
    span.setAttribute("node.driver", node.driver);
    span.setAttribute("node.handler", node.handler);
    span.setAttribute("node.triggers", triggers.length);
    this.spans.push(span);
    this.emit("node:start", node.id, {
      traceparent: span.getTraceparent(),
      driver: node.driver,
      handler: node.handler,
    });
    return span;
  }

  onNodeEnd(node: Node, events: ReadonlyArray<Event>, error: Error | null, span: Span): void {
    span.setAttribute("node.emitted", events.length);
    if (error) span.recordError(error);
    span.end();
    this.emit("node:end", node.id, { emitted: events.length });
  }

  onEvent(event: Event): void {
    this.events.push(event);
    this.emit("event", event.type, { source: event.source });
  }

  onInject(nodeSpecs: ReadonlyArray<Record<string, unknown>>): void {
    this.emit("inject", `${nodeSpecs.length} nodes`, { nodes: nodeSpecs.length });
  }

  onRetire(nodeId: string): void {
    this.retirements.push(nodeId);
    this.emit("retire", nodeId);
  }
}

export const OtelObserverLive = Layer.succeed(Observer, new OtelObserver());
