import http from "node:http";
import { Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "handl-core";
import { resolveValueForNode } from "handl-template-engine";

type PostData = { body: unknown; headers: unknown };

const startServer = (port: number, host: string) =>
  Effect.tryPromise<http.Server, DriverError>({
    try: () =>
      new Promise((resolve, reject) => {
        const server = http.createServer();
        const onError = (error: Error) => {
          server.off("error", onError);
          reject(error);
        };
        server.once("error", onError);
        server.listen(port, host, () => {
          server.off("error", onError);
          resolve(server);
        });
      }),
    catch: (error) => new DriverError({ reason: String(error) }),
  });

const closeServer = (server: http.Server) =>
  Effect.tryPromise({
    try: () =>
      new Promise<void>((resolve) => {
        server.close(() => resolve());
      }),
    catch: (error) => new DriverError({ reason: String(error) }),
  }).pipe(Effect.catchAll(() => Effect.succeed(undefined)));

const waitForPost = (server: http.Server, path: string, response: unknown) =>
  Effect.async<PostData, DriverError>((resume) => {
    const onRequest = (req: http.IncomingMessage, res: http.ServerResponse) => {
      if (req.method !== "POST") {
        res.writeHead(405).end();
        return;
      }
      const pathname = new URL(req.url ?? "", "http://x").pathname;
      if (pathname !== path) {
        res.writeHead(404).end();
        return;
      }
      const chunks: Buffer[] = [];
      req.on("data", (chunk) => chunks.push(chunk));
      req.once("end", () => {
        const raw = Buffer.concat(chunks).toString();
        let body: unknown = raw;
        try {
          body = JSON.parse(raw);
        } catch {}
        const responseBody =
          response === undefined
            ? JSON.stringify({ status: "ok" })
            : typeof response === "string"
              ? response
              : JSON.stringify(response);
        const contentType =
          response === undefined || typeof response !== "string"
            ? "application/json"
            : "text/plain";
        res.writeHead(200, { "Content-Type": contentType }).end(responseBody);
        resume(Effect.succeed({ body, headers: req.headers }));
      });
      req.once("error", (error) => {
        res.writeHead(500).end();
        resume(Effect.fail(new DriverError({ reason: String(error) })));
      });
    };
    server.on("request", onRequest);
    return Effect.sync(() => server.off("request", onRequest));
  });

export const WebhookDriver: Driver = {
  name: "webhook",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );
      const params = resolved as Record<string, unknown>;
      const port = typeof params.port === "number" ? params.port : 8080;
      const path = typeof params.path === "string" ? params.path : "/events";
      const host = typeof params.host === "string" ? params.host : "127.0.0.1";
      const response = params.response;
      const timeout = typeof params.timeout === "number" ? params.timeout : 300;
      const timeoutMs = timeout > 0 ? timeout * 1000 : 0;

      if (ctx.dryRun) {
        return [new Event("webhook.listening", { port, path, dryRun: true }, node.id)];
      }

      const serverOrError = yield* startServer(port, host).pipe(
        Effect.match({
          onFailure: (error) => error,
          onSuccess: (server) => server,
        }),
      );
      if (serverOrError instanceof DriverError) {
        return [new Event("node.failed", { error: serverOrError.reason }, node.id)];
      }
      const server = serverOrError as http.Server;

      const post = waitForPost(server, path, response).pipe(Effect.ensuring(closeServer(server)));
      const wait =
        timeoutMs > 0
          ? Effect.raceFirst(
              post,
              Effect.sleep(timeoutMs).pipe(Effect.map(() => "timeout" as const)),
            )
          : post;

      const result = yield* wait.pipe(
        Effect.match({
          onFailure: (error) => error,
          onSuccess: (value) => value,
        }),
      );

      if (result instanceof DriverError) {
        return [new Event("webhook.failed", { error: result.reason }, node.id)];
      }
      if (result === "timeout") {
        return [new Event("webhook.failed", { reason: "timeout" }, node.id)];
      }
      return [
        new Event("webhook.received", { body: result.body, headers: result.headers }, node.id),
      ];
    }),
};

export const WebhookDriverLive = Layer.succeed(Driver, WebhookDriver);
