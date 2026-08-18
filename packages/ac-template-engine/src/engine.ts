import fs from "node:fs";
import path from "node:path";
import cp from "node:child_process";
import { Effect } from "effect";
import * as nunjucks from "nunjucks";
import type { Event, ExecutionContext, Node } from "ac-core";

const knownPrefixes = ["event", "config", "secret", "node", "state"];

const sentinelBase: Record<string | symbol, unknown> = {
  toString: () => "",
  valueOf: () => 0,
  [Symbol.toPrimitive]: () => "",
};
const sentinel: Record<string | symbol, unknown> = new Proxy(sentinelBase, {
  get: (target, prop) => (prop in target ? target[prop] : sentinel),
});

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

const env = new nunjucks.Environment(undefined, {
  autoescape: false,
  throwOnUndefined: false,
  tags: {
    variableStart: "${",
    variableEnd: "}",
  },
});

env.addFilter("file", readFile);
env.addFilter("shell", runShell);

function isJinjaExpr(content: string): boolean {
  const stripped = content.trim();
  if (stripped.includes("|")) return true;
  for (const prefix of knownPrefixes) {
    if (stripped === prefix || stripped.startsWith(prefix + ".")) return true;
  }
  return false;
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

function resolveDefaults(template: string, context: Record<string, unknown>): string {
  const result: string[] = [];
  let i = 0;
  while (i < template.length) {
    if (template[i] === "$" && template[i + 1] === "{") {
      const end = findMatchingBrace(template, i + 1);
      if (end === -1) {
        result.push(template.slice(i));
        break;
      }
      const inner = template.slice(i + 2, end);
      const sep = inner.indexOf(":-");
      if (sep !== -1) {
        const varPart = inner.slice(0, sep).trim();
        if (isJinjaExpr(varPart)) {
          try {
            const value = env.renderString("${" + varPart + "}", context);
            if (value) {
              result.push(value);
            } else {
              result.push(resolveDefaults(inner.slice(sep + 2), context));
            }
          } catch {
            result.push(resolveDefaults(inner.slice(sep + 2), context));
          }
          i = end + 1;
          continue;
        }
        const resolvedDefault = renderTemplate(inner.slice(sep + 2), context);
        result.push("${" + inner.slice(0, sep) + ":-" + resolvedDefault + "}");
        i = end + 1;
        continue;
      }
      result.push(template.slice(i, end + 1));
      i = end + 1;
    } else {
      result.push(template[i]);
      i++;
    }
  }
  return result.join("");
}

function renderTemplate(template: string, context: Record<string, unknown>): string {
  const pre = resolveDefaults(template, context);
  const segments: string[] = [];
  let lastEnd = 0;
  const braceRe = /\$\{([^}]+)\}/g;
  let match: RegExpExecArray | null;
  while ((match = braceRe.exec(pre)) !== null) {
    const content = match[1];
    if (isJinjaExpr(content)) {
      if (match.index > lastEnd) segments.push(pre.slice(lastEnd, match.index));
      try {
        const rendered = env.renderString("${" + content + "}", context);
        segments.push(rendered);
      } catch {
        segments.push(match[0]);
      }
      lastEnd = match.index + match[0].length;
    }
  }
  if (lastEnd < pre.length) segments.push(pre.slice(lastEnd));
  return segments.join("");
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
  template: string,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
): string {
  const context = makeContext(triggers, ctx, node);
  return renderTemplate(template, context);
}

function resolveValueImpl(
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node?: Node,
): unknown {
  if (typeof value === "string") return resolveTemplateImpl(value, triggers, ctx, node);
  if (Array.isArray(value)) return value.map((v) => resolveValueImpl(v, triggers, ctx, node));
  if (value !== null && typeof value === "object") {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value)) out[k] = resolveValueImpl(v, triggers, ctx, node);
    return out;
  }
  return value;
}

export const resolveTemplate = (
  template: string,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
): Effect.Effect<string, unknown> => Effect.try(() => resolveTemplateImpl(template, triggers, ctx));

export const resolveValue = (
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
): Effect.Effect<unknown, unknown> => Effect.try(() => resolveValueImpl(value, triggers, ctx));

export const resolveValueForNode = (
  value: unknown,
  triggers: ReadonlyArray<Event>,
  ctx: ExecutionContext,
  node: Node,
): Effect.Effect<unknown, unknown> =>
  Effect.try(() => resolveValueImpl(value, triggers, ctx, node));
