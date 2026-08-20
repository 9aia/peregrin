import cp from "node:child_process";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, EventSchema, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

export const ShellDriver: Driver = {
  name: "shell",
  input: new EventSchema("", {
    cmd: "string",
    timeout: "integer?",
    emit: "object?",
    env: "object?",
  }),
  output: [
    new EventSchema("cmd.done", { stdout: "string", rc: "integer" }),
    new EventSchema("node.failed", {
      error: "string",
      rc: "integer",
      stdout: "string?",
      stderr: "string?",
    }),
  ],
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.with_, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const with_ = resolved as Record<string, unknown>;
      const cmd = with_.cmd;
      if (typeof cmd !== "string" || cmd.length === 0) {
        return yield* Effect.fail(
          new DriverError({ reason: `shell node '${node.id}' missing 'with.cmd'` }),
        );
      }

      if (ctx.dryRun) {
        return [new Event("cmd.done", { cmd, dryRun: true, stdout: "" }, node.id)];
      }

      const env = with_.env as Record<string, string> | undefined;
      const result = yield* Effect.sync(() =>
        cp.spawnSync(cmd, {
          shell: true,
          encoding: "utf-8",
          timeout: 30000,
          env: env ? { ...process.env, ...env } : process.env,
        }),
      );

      if (result.error) {
        return [new Event("node.failed", { error: String(result.error), cmd, rc: -1 }, node.id)];
      }

      const rc = result.status ?? -1;
      const stdout = result.stdout ?? "";
      const stderr = result.stderr ?? "";

      const emitMap = with_.emit as Record<string, string> | undefined;
      const rcKey = String(rc);
      const mapEvent =
        emitMap && typeof emitMap === "object" && !Array.isArray(emitMap)
          ? Object.hasOwn(emitMap, rcKey)
            ? emitMap[rcKey]
            : Object.hasOwn(emitMap, "default")
              ? emitMap["default"]
              : undefined
          : undefined;

      if (mapEvent !== undefined) {
        return [new Event(mapEvent, { stdout, rc, cmd }, node.id)];
      }

      if (rc === 0) {
        return [new Event("cmd.done", { stdout, rc, cmd }, node.id)];
      }

      return [new Event("node.failed", { stdout, stderr, rc, cmd }, node.id)];
    }),
};

export const ShellDriverLive = Layer.succeed(Driver, ShellDriver);
