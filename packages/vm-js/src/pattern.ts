import { Context } from "./context.js";

export type Pattern =
  | { type: "identifier"; name: string }
  | { type: "array"; elements: Pattern[] }
  | { type: "object"; entries: { key: string; pattern: Pattern }[] }
  | { type: "rest"; name: string };

export function bindPattern(ctx: Context, pattern: Pattern, value: unknown): void {
  switch (pattern.type) {
    case "identifier":
      ctx.define(pattern.name, value);
      break;
    case "array": {
      if (!Array.isArray(value)) throw new Error("cannot destructure non-array");
      for (let i = 0; i < pattern.elements.length; i++) {
        bindPattern(ctx, pattern.elements[i], value[i]);
      }
      break;
    }
    case "object": {
      if (typeof value !== "object" || value === null)
        throw new Error("cannot destructure non-object");
      const record = value as Record<string, unknown>;
      for (const { key, pattern: p } of pattern.entries) {
        bindPattern(ctx, p, record[key]);
      }
      break;
    }
    case "rest":
      ctx.define(pattern.name, value);
      break;
  }
}
