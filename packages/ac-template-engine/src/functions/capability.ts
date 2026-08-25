import { Effect } from "effect";
import { TemplateError } from "handl-core";

const specials = new Set(["\\", "^", "$", ".", "|", "?", "*", "+", "(", ")", "[", "]", "{", "}"]);

function globMatch(pattern: string, value: string): boolean {
  let out = "^";
  for (const c of pattern) {
    if (c === "*") out += ".*";
    else if (c === "?") out += ".";
    else if (specials.has(c)) out += "\\" + c;
    else out += c;
  }
  out += "$";
  return new RegExp(out).test(value);
}

function asList(value: unknown): Array<string> {
  if (!Array.isArray(value)) return [];
  return value.map((v) => (typeof v === "string" ? v : ""));
}

function matches(value: string, caps: unknown): boolean {
  if (caps === true) return true;
  if (caps === false || caps === undefined || caps === null) return false;
  if (Array.isArray(caps)) return caps.some((pattern) => globMatch(String(pattern), value));
  if (typeof caps === "object") {
    const c = caps as Readonly<Record<string, unknown>>;
    for (const pattern of asList(c["deny"])) {
      if (globMatch(pattern, value)) return false;
    }
    const allow = asList(c["allow"]);
    if (allow.length === 0) return false;
    return allow.some((pattern) => globMatch(pattern, value));
  }
  return false;
}

export function checkCapability(
  name: string,
  value: string,
  strict: boolean,
  caps: Readonly<Record<string, unknown>>,
): Effect.Effect<void, TemplateError> {
  if (!strict) return Effect.succeed(undefined);
  const c = caps[name];
  if (c === undefined || c === false || !matches(value, c)) {
    return Effect.fail(
      new TemplateError({
        reason: `capability '${name}' not granted for '${value}'`,
      }),
    );
  }
  return Effect.succeed(undefined);
}
