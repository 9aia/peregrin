import fs from "node:fs";
import path from "node:path";
import cp from "node:child_process";
import { Effect } from "effect";
import { template } from "lodash-es";
import type { ExecutionContext } from "ac-core";
import { Event, Node, TemplateError } from "ac-core";

const knownPrefixes = ["event", "config", "secret", "node", "state", "file", "shell"];

function readFile(filePath: string): string {
  const resolved = path.isAbsolute(filePath) ? filePath : path.join(process.cwd(), filePath);
  try {
    const buffer = fs.readFileSync(resolved);
    return new TextDecoder("utf-8", { fatal: false }).decode(buffer);
  } catch {
    return "";
  }
}

function runShell(cmd: string): string {
  try {
    const output = cp.execSync(cmd, { encoding: "utf8" });
    return output.replace(/\s+$/, "");
  } catch {
    return "";
  }
}

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

const templateSettings = {
  escape: /[^\s\S]/g,
  evaluate: /[^\s\S]/g,
  interpolate: /\$\{([\s\S]+?)\}/g,
  imports: { __s: toStringSafe, file: readFile, shell: runShell },
};

function isJinjaExpr(content: string): boolean {
  const stripped = content.trim();
  if (stripped.includes("|")) return false;
  for (const prefix of knownPrefixes) {
    if (stripped === prefix || stripped.startsWith(prefix + ".")) return true;
    if (stripped.startsWith(prefix + "(")) return true;
  }
  return false;
}

function resolvePlaceholder(placeholder: string, context: Record<string, unknown>): string {
  if (!placeholder.startsWith("${") || !placeholder.endsWith("}")) return placeholder;
  const content = placeholder.slice(2, -1);
  if (!isJinjaExpr(content)) return placeholder;
  try {
    const compiled = template(`__s(${placeholder})`, templateSettings);
    return compiled(context);
  } catch {
    return placeholder;
  }
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

function resolveDefaults(input: string, context: Record<string, unknown>): string {
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
        if (isJinjaExpr(varPart)) {
          const resolved = resolvePlaceholder("${" + varPart + "}", context);
          if (resolved) {
            result.push(resolved);
          } else {
            result.push(resolveDefaults(defaultPart, context));
          }
        } else {
          result.push("${" + varPart + ":-");
          result.push(resolveDefaults(defaultPart, context));
          result.push("}");
        }
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

function renderTemplate(input: string, context: Record<string, unknown>): string {
  const re = /\$\{([^}]+)\}/g;
  const segments: string[] = [];
  let lastEnd = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(input)) !== null) {
    if (match.index > lastEnd) segments.push(input.slice(lastEnd, match.index));
    segments.push(resolvePlaceholder(match[0], context));
    lastEnd = match.index + match[0].length;
  }
  if (lastEnd < input.length) segments.push(input.slice(lastEnd));
  return segments.join("");
}

function findAllPlaceholders(input: string): ReadonlyArray<string> {
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
      if (!isJinjaExpr(inner)) found.push(input.slice(i, end + 1));
      i = end + 1;
    } else {
      i++;
    }
  }
  return found;
}

function captureWildcards(pattern: string, eventType: string): Record<string, string> | null {
  const ps = pattern.split(".");
  const es = eventType.split(".");
  const caps: Record<string, string> = {};
  function backtrack(i: number, j: number, n: number): boolean {
    if (i === ps.length && j === es.length) return true;
    if (i === ps.length || j > es.length) return false;
    const p = ps[i];
    if (p === "**") {
      const max = es.length - (ps.length - i - 1) - j;
      for (let k = 0; k <= max; k++) {
        const value = es.slice(j, j + k).join(".");
        caps[String(n)] = value;
        if (n === 1) caps["shard"] = value;
        if (backtrack(i + 1, j + k, n + 1)) return true;
      }
      return false;
    }
    if (p === "*") {
      if (j >= es.length) return false;
      const value = es[j];
      caps[String(n)] = value;
      if (n === 1) caps["shard"] = value;
      return backtrack(i + 1, j + 1, n + 1);
    }
    if (es[j] !== p) return false;
    return backtrack(i + 1, j + 1, n);
  }
  return backtrack(0, 0, 1) ? caps : null;
}

function eventCapturesForNode(triggers: ReadonlyArray<Event>, node: Node): Record<string, string> {
  if (triggers.length === 0) return {};
  const eventType = triggers[0].type;
  for (const pattern of node.on) {
    const caps = captureWildcards(pattern, eventType);
    if (caps !== null) return caps;
  }
  return {};
}

const sentinel: Record<string | symbol, unknown> = new Proxy(
  {
    toString: () => "",
    valueOf: () => 0,
    [Symbol.toPrimitive]: () => "",
  },
  {
    get: (target, prop) => (prop in target ? Reflect.get(target, prop) : sentinel),
  },
);

function makeContext(
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
): Record<string, unknown> {
  const rawEvent = triggers[0];
  const captures = node ? eventCapturesForNode(triggers, node) : {};
  const event: unknown =
    Object.keys(captures).length > 0 && rawEvent
      ? new Proxy(rawEvent, {
          get: (target, prop) => {
            if (prop === "shard") return captures["shard"] ?? "";
            if (prop === "match") return captures;
            return Reflect.get(target, prop);
          },
        })
      : (rawEvent ?? sentinel);
  const nodeView: unknown = node ? { ...node.toDict(), fires: ctx.fires } : sentinel;
  return {
    event,
    config: ctx.config,
    secret: ctx.secrets,
    node: nodeView,
    state: ctx.state.toRecord(),
  };
}

function resolveTemplateImpl(
  templateStr: string,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
): string {
  const context = makeContext(triggers, ctx, node);
  const pre = resolveDefaults(templateStr, context);
  return renderTemplate(pre, context);
}

export const resolveTemplate = (
  templateStr: string,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
): Effect.Effect<string, TemplateError> =>
  Effect.gen(function* () {
    const rendered = yield* Effect.sync(() =>
      resolveTemplateImpl(templateStr, triggers, ctx, node),
    );
    if (ctx.strict) {
      const unresolved = findAllPlaceholders(rendered);
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
): Effect.Effect<unknown, TemplateError> {
  return Effect.gen(function* () {
    if (typeof value === "string") {
      return yield* resolveTemplate(value, triggers, ctx, node);
    }
    if (Array.isArray(value)) {
      const out: Array<unknown> = [];
      for (const v of value) {
        out.push(yield* resolveValueEffect(v, triggers, ctx, node));
      }
      return out;
    }
    if (value !== null && typeof value === "object") {
      const out: Record<string, unknown> = {};
      for (const [k, v] of Object.entries(value)) {
        out[k] = yield* resolveValueEffect(v, triggers, ctx, node);
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
): Effect.Effect<unknown, TemplateError> => resolveValueEffect(value, triggers, ctx);

export const resolveValueForNode = (
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node: Node,
): Effect.Effect<unknown, TemplateError> => resolveValueEffect(value, triggers, ctx, node);
