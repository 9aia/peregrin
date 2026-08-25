import { Effect, Layer } from "effect";
import { Driver, Event } from "handl-core";
import type { DriverError, ExecutionContext, Node } from "handl-core";

export class NoopDriver implements Driver {
  readonly name = "noop";

  readonly invoke = (
    node: Node,
    _triggers: ReadonlyArray<Event>,
    _ctx: ExecutionContext,
  ): Effect.Effect<ReadonlyArray<Event>, DriverError> =>
    Effect.succeed<ReadonlyArray<Event>>([new Event("node.done", {}, node.id)]);
}

export const NoopDriverLive = Layer.succeed(Driver, new NoopDriver());
