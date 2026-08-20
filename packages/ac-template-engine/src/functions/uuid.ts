import crypto from "node:crypto";
import { Effect } from "effect";
import type { TemplateFunction } from "./types.js";

export const uuidFunction: TemplateFunction = {
  name: "uuid",
  deterministic: false,
  requiresCapability: false,
  call() {
    return Effect.sync(() => crypto.randomUUID());
  },
};
