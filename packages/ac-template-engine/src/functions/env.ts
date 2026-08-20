import { Effect } from "effect";
import { TemplateError } from "ac-core";
import type { TemplateFunction } from "./types.js";
import { checkCapability } from "./capability.js";

export const envFunction: TemplateFunction = {
  name: "env",
  deterministic: true,
  requiresCapability: true,
  call(args, ctx) {
    const name = args[0];
    if (typeof name !== "string") {
      return Effect.fail(new TemplateError({ reason: "env() requires a string name" }));
    }
    return Effect.gen(function* () {
      yield* checkCapability("env", name, ctx.strict, ctx.capabilities);
      const value = process.env[name];
      if (value === undefined) {
        return args.length > 1 ? args[1] : "";
      }
      return value;
    });
  },
};
