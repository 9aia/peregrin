import crypto from "node:crypto";
import { Layer, Option } from "effect";
import { Observer, type Span, type Workflow, type Node, type Event } from "ac-core";

function generateTraceparent(): string {
  const traceId = crypto.randomBytes(16).toString("hex");
  const spanId = crypto.randomBytes(8).toString("hex");
  return `00-${traceId}-${spanId}-01`;
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
    console.log(
      `[otel:span:end] ${this.name} traceparent=${this.traceparent} attributes=${JSON.stringify(this.attributes)}`,
    );
  }

  setAttribute(key: string, value: unknown): void {
    this.attributes[key] = value;
  }

  recordError(error: Error): void {
    this.setAttribute("error", `${error.name}: ${error.message}`);
    console.log(`[otel:span:error] ${this.name} ${error.name}: ${error.message}`);
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

  onWorkflowStart(workflow: Workflow): void {
    const span = new OtelSpan(`workflow:${workflow.name}`);
    span.setAttribute("workflow.name", workflow.name);
    span.setAttribute("workflow.nodes", workflow.nodes.length);
    this.spans.push(span);
    this.workflowSpan = Option.some(span);
    console.log(`[otel:workflow:start] ${workflow.name}`);
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
        console.log(
          `[otel:workflow:end] ${span.name} events=${eventCount} cancelled=${cancelled} cycle=${cycle} failed=${failed}`,
        );
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
    console.log(`[otel:node:start] ${node.id} traceparent=${span.getTraceparent()}`);
    return span;
  }

  onNodeEnd(node: Node, events: ReadonlyArray<Event>, error: Error | null, span: Span): void {
    span.setAttribute("node.emitted", events.length);
    if (error) span.recordError(error);
    span.end();
    console.log(`[otel:node:end] ${node.id} emitted=${events.length}`);
  }

  onEvent(event: Event): void {
    this.events.push(event);
    console.log(`[otel:event] ${event.type}`);
  }

  onInject(nodeSpecs: ReadonlyArray<Record<string, unknown>>): void {
    console.log(`[otel:inject] ${JSON.stringify(nodeSpecs)}`);
  }

  onRetire(nodeId: string): void {
    this.retirements.push(nodeId);
    console.log(`[otel:retire] ${nodeId}`);
  }
}

export const OtelObserverLive = Layer.succeed(Observer, new OtelObserver());
