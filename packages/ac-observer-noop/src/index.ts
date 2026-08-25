import { Observer, type Span, type Workflow, type Node, type Event } from "handl-core";
import { Layer as EffectLayer } from "effect";

export class NoopSpan implements Span {
  end(): void {}

  setAttribute(_key: string, _value: unknown): void {}

  recordError(_error: Error): void {}

  getTraceparent(): string {
    return "";
  }
}

export class NoopObserver implements Observer {
  onWorkflowStart(_workflow: Workflow): void {}

  onWorkflowEnd(
    _eventCount: number,
    _cancelled: boolean,
    _cycle: boolean,
    _failed: boolean,
  ): void {}

  onNodeStart(_node: Node, _triggers: ReadonlyArray<Event>): Span {
    return new NoopSpan();
  }

  onNodeEnd(_node: Node, _events: ReadonlyArray<Event>, _error: Error | null, span: Span): void {
    span.end();
  }

  onEvent(_event: Event): void {}

  onInject(_nodeSpecs: ReadonlyArray<Record<string, unknown>>): void {}

  onRetire(_nodeId: string): void {}
}

export const Live: Observer = new NoopObserver();

export const Layer = EffectLayer.succeed(Observer, Live);
