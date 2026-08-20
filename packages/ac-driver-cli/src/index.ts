import cp from "node:child_process";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";

function buildEnv(params: Record<string, unknown>, ctx: ExecutionContext): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { ...process.env };
  const configEnv = params.env;
  if (configEnv !== null && typeof configEnv === "object") {
    for (const [key, value] of Object.entries(configEnv as Record<string, unknown>)) {
      env[key] = String(value);
    }
  }
  const secrets = params.secrets;
  if (Array.isArray(secrets)) {
    for (const key of secrets) {
      if (typeof key === "string") {
        env[key] = ctx.getSecret(key);
      }
    }
  }
  return env;
}

export const CliDriver: Driver = {
  name: "cli",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const params = resolved as Record<string, unknown>;
      const cmd = params.cmd;
      if (typeof cmd !== "string" || cmd.length === 0) {
        return yield* Effect.fail(
          new DriverError({ reason: `cli node '${node.id}' missing 'params.cmd'` }),
        );
      }

      const interval = typeof params.interval === "number" ? params.interval : 60;
      const parseJson = params.parse_json === true;
      const mode = params.mode === "interval" ? "interval" : "self-trigger";
      const eventType =
        typeof params.event_type === "string" && params.event_type.length > 0
          ? params.event_type
          : (node.emits[0] ?? `${node.id}.line`);
      const tickEvent = `${node.id}.tick`;
      const doneEvent = `${node.id}.done`;

      if (ctx.dryRun) {
        return [new Event(eventType, { cmd, dryRun: true, interval, mode }, node.id)];
      }

      const env = buildEnv(params, ctx);

      const result = yield* Effect.try({
        try: () =>
          cp.spawnSync(cmd, {
            shell: true,
            encoding: "utf-8",
            timeout: 300000,
            env,
          }),
        catch: (error) => new DriverError({ reason: String(error) }),
      });

      if (result.error) {
        return [new Event("node.failed", { error: String(result.error), cmd, rc: -1 }, node.id)];
      }

      const rc = result.status ?? -1;
      const stdout = result.stdout ?? "";
      const events: Array<Event> = [];

      if (mode === "interval") {
        events.push(
          new Event(`${node.id}.listening`, { cmd, interval, mode: "interval" }, node.id),
        );
      }

      for (const raw of stdout.split("\n")) {
        const line = raw.trim();
        if (line.length === 0) continue;

        let payload: Record<string, unknown>;
        let type = eventType;

        if (parseJson) {
          try {
            const parsed = JSON.parse(line) as Record<string, unknown>;
            if ("type" in parsed) {
              const { type: parsedType, ...rest } = parsed;
              type = String(parsedType);
              payload = { ...rest, rc };
            } else {
              payload = { ...parsed, rc };
            }
          } catch {
            payload = { line, rc };
          }
        } else {
          payload = { line, rc };
        }

        events.push(new Event(type, payload, node.id));
      }

      if (mode === "interval") {
        events.push(new Event(doneEvent, { cmd, rc }, node.id));
        return events;
      }

      events.push(new Event(tickEvent, { cmd, interval, rc }, node.id));
      return events;
    }),
};

export const CliDriverLive = Layer.succeed(Driver, CliDriver);
