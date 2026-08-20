export interface ParsedCall {
  readonly name: string;
  readonly method?: string;
  readonly args: ReadonlyArray<unknown>;
}

const CALL_PATTERN = /^([a-zA-Z_]\w*(?:\.[a-zA-Z_]\w*)?)\s*\(\s*(.*?)\s*\)\s*$/s;

export function parseCall(content: string): ParsedCall | null {
  const trimmed = content.trim();
  const match = trimmed.match(CALL_PATTERN);
  if (!match) return null;
  const full = match[1];
  const parts = full.split(".");
  if (parts.length > 2) return null;
  const name = parts[0];
  const method = parts[1];
  const argsText = match[2] ?? "";
  return { name, method, args: parseArgs(argsText) };
}

function parseArgs(text: string): Array<unknown> {
  if (text.trim() === "") return [];
  const out: Array<unknown> = [];
  let current = "";
  let quote: string | null = null;
  for (const ch of text) {
    if (quote !== null) {
      if (ch === quote) quote = null;
      current += ch;
    } else if (ch === '"' || ch === "'") {
      quote = ch;
      current += ch;
    } else if (ch === ",") {
      out.push(parseArg(current));
      current = "";
    } else {
      current += ch;
    }
  }
  out.push(parseArg(current));
  return out;
}

function parseArg(s: string): unknown {
  const t = s.trim();
  if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'"))) {
    return t.slice(1, -1);
  }
  if (t === "true") return true;
  if (t === "false") return false;
  if (t === "null") return null;
  if (t !== "" && /^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?$/.test(t)) return Number(t);
  return t;
}
