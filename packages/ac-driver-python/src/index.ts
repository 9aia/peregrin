import cp from "node:child_process";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

export const PythonDriver: Driver = {
  name: "python",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.with_, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const with_ = resolved as Record<string, unknown>;
      const script = with_.script;
      const module_ = with_.module;
      const function_ = with_.function;
      const args = Array.isArray(with_.args) ? with_.args.map(String) : [];
      const timeout = typeof with_.timeout === "number" ? with_.timeout : 30000;

      if (typeof script === "string" && script.length > 0) {
        if (ctx.dryRun) {
          return [new Event("python.done", { dryRun: true, stdout: "", rc: 0, script }, node.id)];
        }

        const result = yield* Effect.sync(() =>
          cp.spawnSync("python", [script, ...args], { encoding: "utf-8", timeout }),
        );

        return yield* makeEvents(node, result, { script });
      }

      if (
        typeof module_ === "string" &&
        module_.length > 0 &&
        typeof function_ === "string" &&
        function_.length > 0
      ) {
        if (ctx.dryRun) {
          return [
            new Event(
              "python.done",
              { dryRun: true, stdout: "", rc: 0, module: module_, function: function_ },
              node.id,
            ),
          ];
        }

        const code = `from ${module_} import ${function_}; print(${function_}())`;
        const result = yield* Effect.sync(() =>
          cp.spawnSync("python", ["-c", code], { encoding: "utf-8", timeout }),
        );

        return yield* makeEvents(node, result, { module: module_, function: function_ });
      }

      return yield* Effect.fail(
        new DriverError({
          reason: `python node '${node.id}' missing 'with.script' or 'with.module' and 'with.function'`,
        }),
      );
    }),
};

function makeEvents(
  node: Node,
  result: ReturnType<typeof cp.spawnSync>,
  details: Record<string, unknown>,
): Effect.Effect<ReadonlyArray<Event>, DriverError> {
  return Effect.sync(() => {
    if (result.error) {
      return [
        new Event("python.failed", { error: String(result.error), rc: -1, ...details }, node.id),
      ];
    }

    const rc = result.status ?? (result.signal ? -1 : 0);
    const stdout = typeof result.stdout === "string" ? result.stdout : "";
    const stderr = typeof result.stderr === "string" ? result.stderr : "";

    if (rc === 0) {
      return [new Event("python.done", { stdout, rc, ...details }, node.id)];
    }

    return [
      new Event(
        "python.failed",
        { stdout, stderr, rc, signal: result.signal ?? "", ...details },
        node.id,
      ),
    ];
  });
}

export const PythonDriverLive = Layer.succeed(Driver, PythonDriver);
