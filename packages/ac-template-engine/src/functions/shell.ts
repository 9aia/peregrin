import { exec } from "node:child_process";
import { Effect } from "effect";
import { TemplateError } from "handl-core";
import type { TemplateFunction } from "./types.js";
import { checkCapability } from "./capability.js";

export const shellFunction: TemplateFunction = {
  name: "shell",
  deterministic: true,
  requiresCapability: true,
  call(args, ctx) {
    const raw = args[0];
    if (typeof raw !== "string") {
      return Effect.fail(new TemplateError({ reason: "shell() requires a string command" }));
    }
    return Effect.gen(function* () {
      yield* checkCapability("shell", raw, ctx.strict, ctx.capabilities);
      const out = yield* Effect.tryPromise(
        () =>
          new Promise<string>((resolve, reject) => {
            exec(raw, { encoding: "utf8" }, (error, stdout) => {
              if (error) reject(error);
              else resolve(stdout.replace(/\s+$/, ""));
            });
          }),
      ).pipe(Effect.mapError((error) => new TemplateError({ reason: `shell() failed: ${error}` })));
      return out;
    });
  },
};
