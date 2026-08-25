import { Effect } from "effect";
import { TemplateError } from "handl-core";
import type { TemplateFunction } from "./types.js";

export const nowFunction: TemplateFunction = {
  name: "now",
  deterministic: false,
  requiresCapability: false,
  call(_args, _ctx, _node, method = "iso") {
    if (method === "unix") {
      return Effect.sync(() => Math.floor(Date.now() / 1000));
    }
    if (method === "iso") {
      return Effect.sync(() => new Date().toISOString());
    }
    return Effect.fail(
      new TemplateError({
        reason: `now.${method}() is not supported`,
      }),
    );
  },
};
