import { Effect } from "effect";
import type { TemplateFunction } from "./types.js";

export const randomFunction: TemplateFunction = {
  name: "random",
  deterministic: false,
  requiresCapability: false,
  call(args) {
    const min = typeof args[0] === "number" ? args[0] : 0;
    const max = typeof args[1] === "number" ? args[1] : 1;
    if (Number.isInteger(min) && Number.isInteger(max) && max >= min) {
      const value = Math.floor(min + Math.random() * (max - min + 1));
      return Effect.succeed(value);
    }
    const value = min + Math.random() * (max - min);
    return Effect.succeed(value);
  },
};
