import { Either, Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "ac-core";
import { resolveValueForNode } from "ac-template-engine";
import { Apprise, Live as AppriseLive } from "ac-plugin-apprise";

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
      const resolved = yield* resolveValueForNode(node.with_, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const with_ = resolved as Record<string, unknown>;
      const url = with_.url;
      if (typeof url !== "string" || url.length === 0) {
        return yield* Effect.fail(
          new DriverError({ reason: `http node '${node.id}' missing 'with.url'` }),
        );
      }

      const method = typeof with_.method === "string" ? with_.method.toUpperCase() : "GET";
      const rawHeaders = with_.headers;
      const headers: Record<string, string> =
        rawHeaders !== null && typeof rawHeaders === "object" && !Array.isArray(rawHeaders)
          ? { ...(rawHeaders as Record<string, string>) }
          : {};
      const timeout = typeof with_.timeout === "number" ? with_.timeout : 300;
      const body = makeBody(with_.body, headers);

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

      const appriseTarget = with_.apprise;
      if (with_.notify === true && typeof appriseTarget === "string" && appriseTarget.length > 0) {
        const apprise = yield* Apprise;
        const notifyBody =
          typeof with_.message === "string" && with_.message.length > 0
            ? with_.message
            : typeof responseBody === "string"
              ? responseBody
              : String(responseBody);
        yield* apprise.send({
          target: appriseTarget,
          body: notifyBody,
          title:
            typeof with_.title === "string" && with_.title.length > 0 ? with_.title : undefined,
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
