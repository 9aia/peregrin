import { Effect, Layer } from "effect";
import { Driver, Event, type ExecutionContext, type Node } from "ac-core";

export const RunnerDriver: Driver = {
  name: "runner",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      if (triggers.length === 0) {
        return [];
      }

      if (ctx.dryRun) {
        return [
          new Event(node.emits[0] ?? `${node.id}.done`, { dry_run: true, injected: 0 }, node.id),
        ];
      }

      const event = triggers[0];
      const nodes = event.payload.nodes;
      const workflow = event.payload.workflow as Record<string, unknown> | undefined;
      const candidate = Array.isArray(nodes)
        ? nodes
        : Array.isArray(workflow?.nodes)
          ? workflow.nodes
          : [];

      const newNodes = (candidate as Array<unknown>).filter(
        (x): x is Record<string, unknown> => typeof x === "object" && x !== null,
      ) as unknown as ReadonlyArray<Record<string, unknown>>;

      if (newNodes.length > 0) {
        yield* Effect.sync(() => ctx.inject(newNodes));
      }

      return [
        new Event(node.emits[0] ?? `${node.id}.done`, { injected: newNodes.length }, node.id),
      ];
    }),
};

export const RunnerDriverLive = Layer.succeed(Driver, RunnerDriver);
