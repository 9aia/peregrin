import cp from "node:child_process";
import path from "node:path";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

function findPythonImport(ctx: ExecutionContext, alias: string): string | undefined {
  for (const imp of ctx.pythonImports ?? []) {
    if (imp.as === alias) return imp.file;
    if (imp.as.length > 0) continue;
    const base = path.basename(imp.file).replace(/\.py$/i, "");
    if (base === alias) return imp.file;
  }
  return undefined;
}

export const PythonDriver: Driver = {
  name: "python",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const params = resolved as Record<string, unknown>;
      const handlerName =
        typeof node.handler === "string" && node.handler.length > 0
          ? node.handler
          : typeof params.handler === "string"
            ? params.handler
            : "";
      const script = params.script;
      const module_ = params.module;
      const function_ = params.function;
      const args = Array.isArray(params.args) ? params.args.map(String) : [];
      const timeout = typeof params.timeout === "number" ? params.timeout : 30000;

      if (handlerName.includes(":")) {
        const [alias, fn] = handlerName.split(":");
        const file = findPythonImport(ctx, alias);
        if (file === undefined) {
          return yield* Effect.fail(
            new DriverError({
              reason: `python node '${node.id}' handler '${handlerName}' has no matching import`,
            }),
          );
        }
        if (ctx.dryRun) {
          return [new Event("python.done", { dryRun: true, file, alias, function: fn }, node.id)];
        }
        const fileJson = JSON.stringify(file);
        const fnJson = JSON.stringify(fn);
        const code =
          `import importlib.util; spec = importlib.util.spec_from_file_location("${alias}", ${fileJson}); ` +
          `mod = importlib.util.module_from_spec(spec); spec.loader.exec_module(mod); ` +
          `print(getattr(mod, ${fnJson})())`;
        const result = yield* Effect.sync(() =>
          cp.spawnSync("python", ["-c", code], { encoding: "utf-8", timeout }),
        );
        return yield* makeEvents(node, result, { file, alias, function: fn });
      }

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
          reason: `python node '${node.id}' missing 'params.script' or 'params.module' and 'params.function'`,
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
