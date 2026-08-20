import { Effect } from "effect";
import { Event, Node, TemplateError, type ExecutionContext } from "ac-core";
import { parseCall } from "./parse.js";
import { templateFunctions, type TemplateFunction } from "./functions/index.js";

const knownPrefixes = [
  "event",
  "config",
  "secret",
  "node",
  "params",
  "state",
  "file",
  "shell",
  "env",
  "random",
  "uuid",
  "now",
];

function toStringSafe(value: unknown): string {
  if (value == null) return "";
  if (typeof value === "string") return value;
  if (
    typeof value === "number" ||
    typeof value === "boolean" ||
    typeof value === "bigint" ||
    typeof value === "symbol"
  ) {
    return String(value);
  }
  if (
    typeof value === "object" &&
    typeof (value as { toString?: unknown }).toString === "function"
  ) {
    return (value as { toString(): string }).toString();
  }
  return JSON.stringify(value);
}

function findMatchingBrace(text: string, start: number): number {
  let depth = 0;
  for (let i = start; i < text.length; i++) {
    if (text[i] === "{") depth++;
    else if (text[i] === "}") {
      depth--;
      if (depth === 0) return i;
    }
  }
  return -1;
}

function isJinjaExpr(content: string): boolean {
  const stripped = content.trim();
  if (stripped.includes("|")) return false;
  for (const prefix of knownPrefixes) {
    if (
      stripped === prefix ||
      stripped.startsWith(prefix + ".") ||
      stripped.startsWith(prefix + "(")
    ) {
      return true;
    }
  }
  return false;
}

function resolveDefaults(input: string): string {
  const result: string[] = [];
  let i = 0;
  while (i < input.length) {
    if (input[i] === "$" && input[i + 1] === "{") {
      const end = findMatchingBrace(input, i + 1);
      if (end === -1) {
        result.push(input.slice(i));
        break;
      }
      const inner = input.slice(i + 2, end);
      const sep = inner.indexOf(":-");
      if (sep !== -1) {
        const varPart = inner.slice(0, sep).trim();
        const defaultPart = inner.slice(sep + 2);
        result.push("${" + varPart + ":-");
        result.push(resolveDefaults(defaultPart));
        result.push("}");
        i = end + 1;
      } else {
        result.push(input.slice(i, end + 1));
        i = end + 1;
      }
    } else {
      result.push(input[i]);
      i++;
    }
  }
  return result.join("");
}

function allFunctions(
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Readonly<Record<string, TemplateFunction>> {
  if (customFunctions === undefined || Object.keys(customFunctions).length === 0)
    return templateFunctions;
  return { ...templateFunctions, ...customFunctions };
}

function resolvePlaceholderEffect(
  placeholder: string,
  ctx: ExecutionContext,
  node: Node | undefined,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Effect.Effect<string, TemplateError> {
  if (!placeholder.startsWith("${") || !placeholder.endsWith("}")) {
    return Effect.succeed(placeholder);
  }
  const content = placeholder.slice(2, -1);
  const call = parseCall(content);
  if (call === null) return Effect.succeed(placeholder);
  const fns = allFunctions(customFunctions);
  const fn = fns[call.name];
  if (fn === undefined) return Effect.succeed(placeholder);
  const caps = ctx.capabilities;
  if (caps[call.name] === undefined || caps[call.name] === false) {
    return Effect.succeed(placeholder);
  }
  return Effect.gen(function* () {
    const value = yield* fn.call(call.args, ctx, node, call.method);
    return toStringSafe(value);
  });
}

function renderTemplateEffect(
  input: string,
  ctx: ExecutionContext,
  node: Node | undefined,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Effect.Effect<string, TemplateError> {
  const re = /\$\{([^}]+)\}/g;
  const textParts: string[] = [];
  const placeholders: string[] = [];
  let lastEnd = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(input)) !== null) {
    textParts.push(input.slice(lastEnd, match.index));
    placeholders.push(match[0]);
    lastEnd = match.index + match[0].length;
  }
  textParts.push(input.slice(lastEnd));
  return Effect.gen(function* () {
    const resolved: string[] = [];
    for (const p of placeholders) {
      resolved.push(yield* resolvePlaceholderEffect(p, ctx, node, customFunctions));
    }
    let out = "";
    for (let i = 0; i < resolved.length; i++) {
      out += textParts[i] + resolved[i];
    }
    out += textParts[resolved.length];
    return out;
  });
}

function isUnresolvable(
  content: string,
  ctx: ExecutionContext,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): boolean {
  const call = parseCall(content);
  if (call !== null) {
    const fns = allFunctions(customFunctions);
    const caps = ctx.capabilities;
    return !(
      fns[call.name] !== undefined &&
      caps[call.name] !== undefined &&
      caps[call.name] !== false
    );
  }
  return !isJinjaExpr(content);
}

function findAllPlaceholders(
  input: string,
  ctx: ExecutionContext,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): ReadonlyArray<string> {
  const found: string[] = [];
  let i = 0;
  while (i < input.length) {
    if (input[i] === "$" && input[i + 1] === "{") {
      const end = findMatchingBrace(input, i + 1);
      if (end === -1) {
        found.push(input.slice(i));
        break;
      }
      const inner = input.slice(i + 2, end).trim();
      if (isUnresolvable(inner, ctx, customFunctions)) found.push(input.slice(i, end + 1));
      i = end + 1;
    } else {
      i++;
    }
  }
  return found;
}

export const resolveTemplate = (
  templateStr: string,
  _triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Effect.Effect<string, TemplateError> =>
  Effect.gen(function* () {
    const pre = resolveDefaults(templateStr);
    const rendered = yield* renderTemplateEffect(pre, ctx, node, customFunctions);
    if (ctx.strict) {
      const unresolved = findAllPlaceholders(rendered, ctx, customFunctions);
      if (unresolved.length > 0) {
        return yield* Effect.fail(
          new TemplateError({
            reason: `unresolved placeholders: ${unresolved.join(", ")}`,
          }),
        );
      }
      if (ctx.missingSecrets.length > 0) {
        return yield* Effect.fail(
          new TemplateError({
            reason: `missing secrets: ${ctx.missingSecrets.join(", ")}`,
          }),
        );
      }
    }
    return rendered;
  });

function resolveValueEffect(
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Effect.Effect<unknown, TemplateError> {
  return Effect.gen(function* () {
    if (typeof value === "string") {
      return yield* resolveTemplate(value, triggers, ctx, node, customFunctions);
    }
    if (Array.isArray(value)) {
      const out: Array<unknown> = [];
      for (const v of value) {
        out.push(yield* resolveValueEffect(v, triggers, ctx, node, customFunctions));
      }
      return out;
    }
    if (value !== null && typeof value === "object") {
      const out: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(value)) {
        out[k] = yield* resolveValueEffect(v, triggers, ctx, node, customFunctions);
      }
      return out;
    }
    return value;
  });
}

export const resolveValue = (
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Effect.Effect<unknown, TemplateError> =>
  resolveValueEffect(value, triggers, ctx, undefined, customFunctions);

export const resolveValueForNode = (
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node: Node,
  customFunctions?: Readonly<Record<string, TemplateFunction>>,
): Effect.Effect<unknown, TemplateError> =>
  resolveValueEffect(value, triggers, ctx, node, customFunctions);
