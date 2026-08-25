import cp from "node:child_process";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "handl-core";
import { resolveValueForNode } from "handl-template-engine";

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const resolveWith = (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
  resolveValueForNode(node.params, triggers, ctx, node).pipe(
    Effect.mapError((error) => new DriverError({ reason: String(error) })),
  );

const lineEvent = (node: Node): string => node.emits[0] ?? `${node.id}.line`;
const doneEvent = (node: Node): string => `${node.id}.done`;

const parsePayloadForLine = (line: string): Record<string, unknown> => {
  try {
    return { data: JSON.parse(line) as unknown, raw: line };
  } catch {
    return { line };
  }
};

const makeBody = (
  rawBody: unknown,
  json: unknown,
  headers: Record<string, string>,
): string | undefined => {
  if (json !== undefined) {
    if (!headers["Content-Type"] && !headers["content-type"]) {
      headers["Content-Type"] = "application/json";
    }
    return JSON.stringify(json);
  }
  if (rawBody === undefined) return undefined;
  if (typeof rawBody === "string") return rawBody;
  if (typeof rawBody === "object" && rawBody !== null) {
    if (!headers["Content-Type"] && !headers["content-type"]) {
      headers["Content-Type"] = "application/json";
    }
    return JSON.stringify(rawBody);
  }
  if (typeof rawBody === "number" || typeof rawBody === "boolean" || typeof rawBody === "bigint") {
    return String(rawBody);
  }
  return undefined;
};

const shellStream = (node: Node, params: Record<string, unknown>, ctx: ExecutionContext) =>
  Effect.gen(function* () {
    const cmd = params.cmd;
    if (typeof cmd !== "string" || cmd.length === 0) {
      return yield* Effect.fail(
        new DriverError({ reason: `streaming node '${node.id}' missing 'params.cmd'` }),
      );
    }

    if (ctx.dryRun) {
      return [new Event(lineEvent(node), { cmd, dryRun: true, rc: 0 }, node.id)];
    }

    const timeout = typeof params.timeout === "number" ? params.timeout : 300;
    const envOverride = isRecord(params.env) ? (params.env as Record<string, string>) : {};
    const env = { ...process.env, ...envOverride };
    const cwd = typeof params.cwd === "string" ? params.cwd : undefined;

    const result = yield* Effect.tryPromise({
      try: () =>
        new Promise<{ rc: number; stdout: string; stderr: string; timedOut: boolean }>(
          (resolve, reject) => {
            const child = cp.exec(
              cmd,
              { encoding: "utf-8", timeout: timeout * 1000, env, cwd },
              (error, stdout, stderr) => {
                const timedOut = error?.killed === true && error?.signal === "SIGTERM";
                if (error && !timedOut) {
                  reject(error);
                  return;
                }
                const rc = typeof child.exitCode === "number" ? child.exitCode : -1;
                resolve({
                  rc: timedOut ? -1 : rc,
                  stdout: stdout ?? "",
                  stderr: stderr ?? "",
                  timedOut,
                });
              },
            );
          },
        ),
      catch: (error) => new DriverError({ reason: String(error) }),
    });

    const events: Array<Event> = [];
    for (const line of result.stdout.split(/\r?\n/)) {
      if (line.length > 0) {
        events.push(new Event(lineEvent(node), { line }, node.id));
      }
    }

    if (result.timedOut) {
      events.push(new Event(doneEvent(node), { rc: -1, cmd, error: "timeout" }, node.id));
    } else {
      events.push(new Event(doneEvent(node), { rc: result.rc, cmd }, node.id));
    }

    return events;
  });

const httpStream = (node: Node, params: Record<string, unknown>, ctx: ExecutionContext) =>
  Effect.gen(function* () {
    const url = params.url;
    if (typeof url !== "string" || url.length === 0) {
      return yield* Effect.fail(
        new DriverError({ reason: `streaming node '${node.id}' missing 'params.url'` }),
      );
    }

    const method = typeof params.method === "string" ? params.method.toUpperCase() : "GET";

    if (ctx.dryRun) {
      return [new Event(lineEvent(node), { method, url, dryRun: true, status: 200 }, node.id)];
    }

    const rawHeaders = params.headers;
    const headers: Record<string, string> =
      rawHeaders !== null && typeof rawHeaders === "object" && !Array.isArray(rawHeaders)
        ? { ...(rawHeaders as Record<string, string>) }
        : {};
    const timeout = typeof params.timeout === "number" ? params.timeout : 300;
    const body = makeBody(params.body, params.json, headers);

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeout * 1000);

    const response = yield* Effect.tryPromise({
      try: () => fetch(url, { method, headers, body, signal: controller.signal }),
      catch: (error) => new DriverError({ reason: String(error) }),
    }).pipe(Effect.ensuring(Effect.sync(() => clearTimeout(timeoutId))));

    const text = yield* Effect.tryPromise({
      try: () => response.text(),
      catch: (error) => new DriverError({ reason: String(error) }),
    });

    const responseHeaders: Record<string, string> = Object.fromEntries(response.headers.entries());

    const events: Array<Event> = [];
    for (const line of text.split(/\r?\n/)) {
      if (line.length > 0) {
        events.push(new Event(lineEvent(node), parsePayloadForLine(line), node.id));
      }
    }

    events.push(
      new Event(
        doneEvent(node),
        { status: response.status, url, method, headers: responseHeaders },
        node.id,
      ),
    );

    return events;
  });

export const StreamingDriver: Driver = {
  name: "streaming",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveWith(node, triggers, ctx);
      const params = resolved as Record<string, unknown>;

      if (typeof params.cmd === "string" && params.cmd.length > 0) {
        return yield* shellStream(node, params, ctx);
      }

      if (typeof params.url === "string" && params.url.length > 0) {
        return yield* httpStream(node, params, ctx);
      }

      return yield* Effect.fail(
        new DriverError({
          reason: `streaming node '${node.id}' requires 'params.cmd' or 'params.url'`,
        }),
      );
    }),
};

export const StreamingDriverLive = Layer.succeed(Driver, StreamingDriver);
