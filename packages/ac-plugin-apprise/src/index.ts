import { Context, Either, Effect, Layer } from "effect";
import { Driver, DriverError, Event, type ExecutionContext, type Node } from "handl-core";
import { resolveValueForNode } from "handl-template-engine";
import { send as ahhaSend, registerService, type ParsedUrl, type Params } from "@toanalien/ahha";
import nodemailer from "nodemailer";

export interface AppriseMessage {
  readonly target: string;
  readonly body: string;
  readonly title?: string | undefined;
}

export interface Apprise {
  readonly send: (message: AppriseMessage) => Effect.Effect<void, DriverError>;
}

export const Apprise = Context.GenericTag<Apprise>("handl/Apprise");

interface NotificationResult {
  readonly success: boolean;
  readonly target: string;
  readonly title?: string | undefined;
  readonly body: string;
  readonly error?: string | undefined;
}

const isHttpUrl = (target: string): boolean =>
  target.startsWith("http://") || target.startsWith("https://");

function normalizeTarget(target: string): string {
  if (target.startsWith("tgram://")) {
    return `telegram://${target.slice("tgram://".length)}`;
  }
  if (target.startsWith("jsons://")) {
    return `https://${target.slice("jsons://".length)}`;
  }
  if (target.startsWith("json://")) {
    return `http://${target.slice("json://".length)}`;
  }
  return target;
}

async function sendMail(
  config: ParsedUrl,
  message: string,
  params?: Params,
): Promise<{ success: boolean; service: string; timestamp: Date }> {
  const host = config.host;
  if (host === undefined || host.length === 0) {
    throw new Error("mailto target missing host");
  }
  const to =
    params?.to ?? config.query["to"] ?? (config.path.length > 0 ? config.path.join("/") : "");
  if (to.length === 0) {
    throw new Error("mailto target missing 'to' address");
  }

  const port =
    config.port ?? (config.scheme === "mailtos" || config.scheme === "smtps" ? 465 : 587);
  const secure =
    config.scheme === "mailtos" ||
    config.scheme === "smtps" ||
    config.query["mode"] === "ssl" ||
    port === 465;
  const user = config.user ?? "";
  const pass = config.password ?? "";
  const from = params?.from ?? config.query["from"] ?? (user.length > 0 ? `${user}@${host}` : to);

  const transport = nodemailer.createTransport({
    host,
    port,
    secure,
    auth: user.length > 0 ? { user, pass } : undefined,
  });

  await transport.sendMail({
    from,
    to,
    subject: params?.title ?? config.query["subject"] ?? "Notification",
    text: message,
  });

  return { success: true, service: config.scheme, timestamp: new Date() };
}

function registerAhhaServices(): void {
  registerService({
    scheme: "mailto",
    create: (config) => ({
      send: (message, params) => sendMail(config, message, params),
    }),
  });
  registerService({
    scheme: "mailtos",
    create: (config) => ({
      send: (message, params) => sendMail(config, message, params),
    }),
  });
}

registerAhhaServices();

interface NotifyArgs {
  readonly target: string;
  readonly body: string;
  readonly title?: string | undefined;
  readonly timeout?: number | undefined;
  readonly bodyFormat?: string | undefined;
}

const notifyOne = (args: NotifyArgs): Effect.Effect<NotificationResult, DriverError> =>
  Effect.gen(function* () {
    if (args.target.length === 0) {
      return yield* Effect.fail(new DriverError({ reason: "apprise notify missing 'target'" }));
    }

    const target = normalizeTarget(args.target);

    if (isHttpUrl(target)) {
      const timeout = typeof args.timeout === "number" && args.timeout > 0 ? args.timeout : 60;
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), timeout * 1000);

      const body = JSON.stringify({
        body: args.body,
        title: args.title,
        format: args.bodyFormat,
      });

      const request = Effect.tryPromise({
        try: () =>
          fetch(target, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body,
            signal: controller.signal,
          }),
        catch: (error) =>
          new DriverError({
            reason: `apprise notify failed for '${args.target}': ${String(error)}`,
          }),
      }).pipe(Effect.ensuring(Effect.sync(() => clearTimeout(timeoutId))));

      const response = yield* request;

      if (!response.ok) {
        return yield* Effect.fail(
          new DriverError({
            reason: `apprise notify failed with status ${response.status}`,
          }),
        );
      }

      return {
        success: true,
        target: args.target,
        title: args.title,
        body: args.body,
      };
    }

    const params: Params = {
      title: args.title ?? "",
      format: args.bodyFormat ?? "text",
    };

    const result = yield* Effect.tryPromise({
      try: () => ahhaSend(target, args.body, params),
      catch: (error) =>
        new DriverError({
          reason: `apprise notify failed for '${args.target}': ${String(error)}`,
        }),
    });

    if (!result.success) {
      return yield* Effect.fail(
        new DriverError({
          reason: `apprise notify failed for '${args.target}': ${result.error?.message ?? "unknown error"}`,
        }),
      );
    }

    return {
      success: true,
      target: args.target,
      title: args.title,
      body: args.body,
    };
  });

const send = (message: AppriseMessage): Effect.Effect<void, DriverError> =>
  Effect.gen(function* () {
    yield* notifyOne({
      target: message.target,
      body: message.body,
      title: message.title,
    });
    return undefined;
  });

const asString = (value: unknown): string | undefined =>
  typeof value === "string" ? value : undefined;

const asStringArray = (value: unknown): ReadonlyArray<string> | null => {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) {
    const out: Array<string> = [];
    for (const item of value) {
      if (typeof item === "string") out.push(item);
    }
    return out.length > 0 ? out : null;
  }
  return null;
};

export const AppriseDriver: Driver = {
  name: "apprise",
  invoke: (node: Node, triggers: ReadonlyArray<Event>, ctx: ExecutionContext) =>
    Effect.gen(function* () {
      const resolved = yield* resolveValueForNode(node.params, triggers, ctx, node).pipe(
        Effect.mapError((error) => new DriverError({ reason: String(error) })),
      );

      const params = resolved as Record<string, unknown>;
      const handler = node.handler === "send_multi" ? "send_multi" : "send";
      const title = asString(params.title);
      const body = asString(params.body);
      const bodyFormat = asString(params.body_format);
      const timeout =
        typeof params.timeout === "number" && params.timeout > 0 ? params.timeout : undefined;

      if (handler === "send") {
        const target = asString(params.target);
        if (target === undefined) {
          return yield* Effect.fail(
            new DriverError({
              reason: `apprise node '${node.id}' missing 'params.target'`,
            }),
          );
        }
        if (body === undefined) {
          return yield* Effect.fail(
            new DriverError({
              reason: `apprise node '${node.id}' missing 'params.body'`,
            }),
          );
        }

        const result = yield* notifyOne({
          target,
          body,
          title,
          timeout,
          bodyFormat,
        }).pipe(Effect.either);

        if (Either.isLeft(result)) {
          return [
            new Event(
              node.emits[1] ?? "notify.failed",
              { target, body, title, success: false, error: result.left.reason },
              node.id,
            ),
          ];
        }

        const success = result.right.success;
        const eventType = success
          ? (node.emits[0] ?? "notify.sent")
          : (node.emits[1] ?? "notify.failed");
        return [new Event(eventType, { ...result.right, body, title, success }, node.id)];
      }

      const rawTargets = params.targets;
      const targets = asStringArray(rawTargets);
      if (targets === null) {
        return yield* Effect.fail(
          new DriverError({
            reason: `apprise node '${node.id}' missing 'params.targets'`,
          }),
        );
      }
      if (body === undefined) {
        return yield* Effect.fail(
          new DriverError({
            reason: `apprise node '${node.id}' missing 'params.body'`,
          }),
        );
      }

      const results = yield* Effect.forEach(targets, (target) =>
        notifyOne({
          target,
          body,
          title,
          timeout,
          bodyFormat,
        }).pipe(Effect.either),
      );

      const outcomes = results.map((result, index) => {
        const target = targets[index];
        if (Either.isLeft(result)) {
          return {
            success: false,
            target,
            body,
            title,
            error: result.left.reason,
          };
        }
        return { ...result.right, body, title };
      });

      const success = outcomes.every((r) => r.success);
      const eventType = success
        ? (node.emits[0] ?? "notify.sent")
        : (node.emits[1] ?? "notify.failed");

      return [
        new Event(
          eventType,
          {
            targets: outcomes,
            title,
            body,
            success,
            count: outcomes.length,
          },
          node.id,
        ),
      ];
    }),
};

export const AppriseDriverLive = Layer.succeed(Driver, AppriseDriver);

export const Live = Layer.succeed(Apprise, {
  send: (message) => send(message),
});
