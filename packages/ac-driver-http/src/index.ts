import { Either, Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "handl-core";
import { resolveValueForNode } from "handl-template-engine";
import { Apprise, Live as AppriseLive } from "handl-plugin-apprise";

const makeBody = (body: unknown, headers: Record<string, string>): string | undefined => {
  if (body === undefined) return undefined;
  if (typeof body === "string") return body;
  if (typeof body === "object" && body !== null) {
    if (!headers["Content-Type"] && !headers["content-type"]) {
      headers["Content-Type"] = "application/json";
    }
    return JSON.stringify(body);
  }
  if (typeof body === "number" || typeof body === "boolean" || typeof body === "bigint") {
    return String(body);
  }
  return undefined;
};

const readBody = (response: Response): Effect.Effect<unknown, DriverError> =>
  Effect.gen(function* () {
    const text = yield* Effect.tryPromise({
      try: () => response.text(),
      catch: (error) => new DriverError({ reason: String(error) }),
    });
    const contentType = response.headers.get("content-type") ?? "";
    if (contentType.includes("application/json")) {
      const parsed = yield* Effect.try(() => JSON.parse(text) as unknown).pipe(
        Effect.orElseSucceed(() => text),
      );
      return parsed;
    }
    return text;
  });

export const HttpDriver: Driver = {
  name: "http",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const params = resolved as Record<string, unknown>;
      const url = params.url;
      if (typeof url !== "string" || url.length === 0) {
        return yield* Effect.fail(
          new DriverError({ reason: `http node '${node.id}' missing 'params.url'` }),
        );
      }

      const method = typeof params.method === "string" ? params.method.toUpperCase() : "GET";
      const rawHeaders = params.headers;
      const headers: Record<string, string> =
        rawHeaders !== null && typeof rawHeaders === "object" && !Array.isArray(rawHeaders)
          ? { ...(rawHeaders as Record<string, string>) }
          : {};
      const timeout = typeof params.timeout === "number" ? params.timeout : 300;
      const body = makeBody(params.body, headers);

      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeout * 1000);

      const request = Effect.tryPromise({
        try: () => fetch(url, { method, headers, body, signal: controller.signal }),
        catch: (error) => new DriverError({ reason: String(error) }),
      }).pipe(Effect.ensuring(Effect.sync(() => clearTimeout(timeoutId))));

      const result = yield* Effect.either(request);

      if (Either.isLeft(result)) {
        return [
          new Event("http.failed", { status: 0, error: result.left.reason, url, method }, node.id),
        ];
      }

      const response = result.right;
      const responseBody = yield* readBody(response);
      const responseHeaders: Record<string, string> = Object.fromEntries(
        response.headers.entries(),
      );

      const payload = {
        status: response.status,
        body: responseBody,
        url,
        method,
        headers: responseHeaders,
      };

      const appriseTarget = params.apprise;
      if (params.notify === true && typeof appriseTarget === "string" && appriseTarget.length > 0) {
        const apprise = yield* Apprise;
        const notifyBody =
          typeof params.message === "string" && params.message.length > 0
            ? params.message
            : typeof responseBody === "string"
              ? responseBody
              : String(responseBody);
        yield* apprise.send({
          target: appriseTarget,
          body: notifyBody,
          title:
            typeof params.title === "string" && params.title.length > 0 ? params.title : undefined,
        });

        return [
          new Event(
            response.ok ? "http.done" : "http.failed",
            { ...payload, notified: true },
            node.id,
          ),
        ];
      }

      if (response.ok) {
        return [new Event("http.done", payload, node.id)];
      }

      return [new Event("http.failed", payload, node.id)];
    }).pipe(Effect.provide(AppriseLive)),
};

export const HttpDriverLive = Layer.succeed(Driver, HttpDriver);
