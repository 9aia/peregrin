import fs from "node:fs";
import path from "node:path";
import { Effect } from "effect";
import { TemplateError } from "handl-core";
import type { TemplateFunction } from "./types.js";
import { checkCapability } from "./capability.js";

export const fileFunction: TemplateFunction = {
  name: "file",
  deterministic: true,
  requiresCapability: true,
  call(args, ctx) {
    const raw = args[0];
    if (typeof raw !== "string") {
      return Effect.fail(new TemplateError({ reason: "file() requires a string path" }));
    }
    return Effect.gen(function* () {
      const resolved = path.isAbsolute(raw) ? raw : path.join(process.cwd(), raw);
      yield* checkCapability("file", resolved, ctx.strict, ctx.capabilities);
      const buffer = yield* Effect.tryPromise(() => fs.promises.readFile(resolved)).pipe(
        Effect.mapError((error) => new TemplateError({ reason: `file() failed: ${error}` })),
      );
      return new TextDecoder("utf-8", { fatal: false }).decode(buffer);
    });
  },
};
