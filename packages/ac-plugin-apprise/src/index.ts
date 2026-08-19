import cp from "node:child_process";
import { Context, Effect, Layer } from "effect";
import { DriverError } from "ac-core";

export interface AppriseMessage {
  readonly target: string;
  readonly body: string;
  readonly title?: string | undefined;
}

export interface Apprise {
  readonly send: (message: AppriseMessage) => Effect.Effect<void, DriverError>;
}

export const Apprise = Context.GenericTag<Apprise>("ac/Apprise");

const send = (message: AppriseMessage): Effect.Effect<void, DriverError> =>
  Effect.gen(function* () {
    const args = [message.target, "--body", message.body];
    if (message.title && message.title.length > 0) {
      args.push("--title", message.title);
    }

    const result = yield* Effect.try({
      try: () =>
        cp.spawnSync("apprise", args, {
          encoding: "utf-8",
          timeout: 60000,
        }),
      catch: (error) =>
        new DriverError({
          reason: `apprise spawn failed: ${String(error)}`,
        }),
    });

    if (result.error) {
      return yield* Effect.fail(
        new DriverError({
          reason: `apprise failed: ${String(result.error)}`,
        }),
      );
    }

    if (result.status !== 0) {
      const stderr = result.stderr ?? "";
      return yield* Effect.fail(
        new DriverError({
          reason: `apprise exited with status ${result.status}: ${stderr}`,
        }),
      );
    }

    return undefined;
  });

export const Live = Layer.succeed(Apprise, {
  send: (message) => send(message),
});
