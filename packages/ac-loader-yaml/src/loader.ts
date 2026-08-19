import fs from "node:fs";
import path from "node:path";
import { parse } from "yaml";
import { Data, Effect } from "effect";
import { Workflow } from "ac-core";
import { discover } from "ac-file-router";

export class LoaderError extends Data.TaggedError("LoaderError")<{
  readonly reason: string;
  readonly cause?: unknown;
}> {}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const readTextFile = (filePath: string): Effect.Effect<string, LoaderError, never> =>
  Effect.try(() => fs.readFileSync(filePath, "utf8")).pipe(
    Effect.mapError(
      (error) => new LoaderError({ reason: `Failed to read file: ${filePath}`, cause: error }),
    ),
  );

const parseYaml = (text: string): Effect.Effect<unknown, LoaderError, never> =>
  Effect.try(() => parse(text) as unknown).pipe(
    Effect.mapError((error) => new LoaderError({ reason: "Failed to parse YAML", cause: error })),
  );

const parseJson = (text: string): Effect.Effect<unknown, LoaderError, never> =>
  Effect.try(() => JSON.parse(text)).pipe(
    Effect.mapError((error) => new LoaderError({ reason: "Failed to parse JSON", cause: error })),
  );

const deepMerge = (local: unknown, imported: unknown): unknown => {
  if (isRecord(local) && isRecord(imported)) {
    const result: Record<string, unknown> = { ...imported };
    for (const [key, value] of Object.entries(local)) {
      if (key in result) {
        result[key] = deepMerge(value, result[key]);
      } else {
        result[key] = value;
      }
    }
    return result;
  }
  if (Array.isArray(local) && Array.isArray(imported)) {
    return [...local, ...imported];
  }
  return local;
};

const resolveIncludeDict = (
  value: unknown,
  baseDir: string,
  seen: Set<string>,
): Effect.Effect<unknown, LoaderError, never> =>
  Effect.gen(function* () {
    if (isRecord(value)) {
      const resolved: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        resolved[key] = yield* resolveIncludeDict(val, baseDir, seen);
      }
      if ("include" in resolved) {
        const includeValue = resolved.include;
        delete resolved.include;
        const paths = Array.isArray(includeValue) ? includeValue : [includeValue];
        let merged: Record<string, unknown> = resolved;
        for (const includePath of paths) {
          if (typeof includePath !== "string") {
            return yield* Effect.fail(new LoaderError({ reason: "include path must be a string" }));
          }
          const fullPath = path.isAbsolute(includePath)
            ? includePath
            : path.normalize(path.join(baseDir, includePath));
          if (seen.has(fullPath)) {
            return yield* Effect.fail(
              new LoaderError({ reason: `circular include detected: ${fullPath}` }),
            );
          }
          seen.add(fullPath);
          const text = yield* readTextFile(fullPath);
          const parsed = yield* parseYaml(text);
          const imported = yield* resolveIncludeDict(parsed, path.dirname(fullPath), seen);
          seen.delete(fullPath);
          merged = deepMerge(merged, imported) as Record<string, unknown>;
        }
        return merged;
      }
      return resolved;
    }
    if (Array.isArray(value)) {
      const out: unknown[] = [];
      for (const item of value) {
        out.push(yield* resolveIncludeDict(item, baseDir, seen));
      }
      return out;
    }
    return value;
  });

const INCLUDE_RE = /\$\{include\(([^)]+)\)\}/g;

const resolveIncludeValue = (
  match: RegExpExecArray,
  baseDir: string,
  seen: Set<string>,
): Effect.Effect<unknown, LoaderError, never> =>
  Effect.gen(function* () {
    const includePath = match[1].trim();
    const fullPath = path.isAbsolute(includePath)
      ? includePath
      : path.normalize(path.join(baseDir, includePath));
    if (seen.has(fullPath)) {
      return yield* Effect.fail(
        new LoaderError({ reason: `circular include detected: ${fullPath}` }),
      );
    }
    seen.add(fullPath);
    const text = yield* readTextFile(fullPath);
    const parsed = yield* parseYaml(text);
    let content = yield* resolveIncludeDict(parsed, path.dirname(fullPath), seen);
    content = yield* resolveIncludeStrings(content, path.dirname(fullPath), seen);
    seen.delete(fullPath);
    return content;
  });

const resolveIncludeStrings = (
  value: unknown,
  baseDir: string,
  seen: Set<string>,
): Effect.Effect<unknown, LoaderError, never> =>
  Effect.gen(function* () {
    if (typeof value === "string") {
      const fullMatch = /^\$\{include\(([^)]+)\)\}$/.exec(value.trim());
      if (fullMatch) {
        return yield* resolveIncludeValue(fullMatch, baseDir, seen);
      }
      const matches: Array<RegExpExecArray> = [];
      const re = new RegExp(INCLUDE_RE);
      let match: RegExpExecArray | null;
      while ((match = re.exec(value)) !== null) {
        matches.push(match);
      }
      let result = value;
      for (let i = matches.length - 1; i >= 0; i--) {
        const m = matches[i];
        const val = yield* resolveIncludeValue(m, baseDir, seen);
        const str = typeof val === "string" ? val : String(val);
        result = result.slice(0, m.index) + str + result.slice(m.index + m[0].length);
      }
      return result;
    }
    if (isRecord(value)) {
      const out: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        const resolvedKey =
          typeof key === "string"
            ? ((yield* resolveIncludeStrings(key, baseDir, seen)) as string)
            : key;
        out[resolvedKey] = yield* resolveIncludeStrings(val, baseDir, seen);
      }
      return out;
    }
    if (Array.isArray(value)) {
      const out: unknown[] = [];
      for (const item of value) {
        out.push(yield* resolveIncludeStrings(item, baseDir, seen));
      }
      return out;
    }
    return value;
  });

const resolveSnippetDefinitions = (
  snippets: Record<string, unknown>,
): Effect.Effect<Record<string, unknown>, LoaderError, never> =>
  Effect.gen(function* () {
    const resolved: Record<string, unknown> = {};
    for (const [name, body] of Object.entries(snippets)) {
      resolved[name] = isRecord(body) ? { ...body } : body;
    }
    for (let i = 0; i <= Object.keys(resolved).length; i++) {
      let changed = false;
      for (const [name, body] of Object.entries(resolved)) {
        if (!isRecord(body) || !("snippet" in body)) continue;
        const local = { ...body };
        const parentName = local.snippet;
        delete local.snippet;
        if (typeof parentName !== "string") {
          return yield* Effect.fail(
            new LoaderError({ reason: `snippet '${name}' references non-string parent` }),
          );
        }
        if (!(parentName in resolved)) {
          return yield* Effect.fail(
            new LoaderError({
              reason: `snippet '${name}' references unknown snippet '${parentName}'`,
            }),
          );
        }
        const parent = resolved[parentName];
        if (!isRecord(parent)) {
          return yield* Effect.fail(
            new LoaderError({ reason: `snippet '${name}' inherits from non-dict '${parentName}'` }),
          );
        }
        if (parentName === name) {
          return yield* Effect.fail(
            new LoaderError({ reason: `circular snippet: '${name}' inherits from itself` }),
          );
        }
        resolved[name] = deepMerge(local, parent);
        changed = true;
      }
      if (!changed) break;
    }
    return resolved;
  });

const resolveSnippetRefs = (
  value: unknown,
  snippets: Record<string, unknown>,
  seen: Set<string>,
): Effect.Effect<unknown, LoaderError, never> =>
  Effect.gen(function* () {
    if (isRecord(value)) {
      const resolved: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        resolved[key] = yield* resolveSnippetRefs(val, snippets, seen);
      }
      if ("snippet" in resolved) {
        const name = resolved.snippet;
        delete resolved.snippet;
        if (typeof name !== "string") {
          return yield* Effect.fail(new LoaderError({ reason: "snippet key must be a string" }));
        }
        if (!(name in snippets)) {
          return yield* Effect.fail(new LoaderError({ reason: `unknown snippet '${name}'` }));
        }
        if (seen.has(name)) {
          return yield* Effect.fail(
            new LoaderError({ reason: `circular snippet reference: '${name}'` }),
          );
        }
        seen.add(name);
        const body = snippets[name];
        if (!isRecord(body)) {
          seen.delete(name);
          return yield* Effect.fail(new LoaderError({ reason: `snippet '${name}' is not a dict` }));
        }
        let merged = deepMerge(resolved, body) as Record<string, unknown>;
        merged = (yield* resolveSnippetRefs(merged, snippets, seen)) as Record<string, unknown>;
        seen.delete(name);
        return merged;
      }
      return resolved;
    }
    if (Array.isArray(value)) {
      const out: unknown[] = [];
      for (const item of value) {
        out.push(yield* resolveSnippetRefs(item, snippets, seen));
      }
      return out;
    }
    return value;
  });

const SNIPPET_RE = /\$\{snippet\(([^)]+)\)\}/g;

const resolveSnippetValue = (
  match: RegExpExecArray,
  snippets: Record<string, unknown>,
  seen: Set<string>,
): Effect.Effect<unknown, LoaderError, never> =>
  Effect.gen(function* () {
    const name = match[1].trim();
    if (!(name in snippets)) {
      return yield* Effect.fail(new LoaderError({ reason: `unknown snippet '${name}'` }));
    }
    if (seen.has(name)) {
      return yield* Effect.fail(
        new LoaderError({ reason: `circular snippet reference: '${name}'` }),
      );
    }
    seen.add(name);
    let content = snippets[name];
    content = yield* resolveSnippetStrings(content, snippets, seen);
    seen.delete(name);
    return content;
  });

const resolveSnippetStrings = (
  value: unknown,
  snippets: Record<string, unknown>,
  seen: Set<string>,
): Effect.Effect<unknown, LoaderError, never> =>
  Effect.gen(function* () {
    if (typeof value === "string") {
      const fullMatch = /^\$\{snippet\(([^)]+)\)\}$/.exec(value.trim());
      if (fullMatch) {
        return yield* resolveSnippetValue(fullMatch, snippets, seen);
      }
      const matches: Array<RegExpExecArray> = [];
      const re = new RegExp(SNIPPET_RE);
      let match: RegExpExecArray | null;
      while ((match = re.exec(value)) !== null) {
        matches.push(match);
      }
      let result = value;
      for (let i = matches.length - 1; i >= 0; i--) {
        const m = matches[i];
        const val = yield* resolveSnippetValue(m, snippets, seen);
        const str = typeof val === "string" ? val : String(val);
        result = result.slice(0, m.index) + str + result.slice(m.index + m[0].length);
      }
      return result;
    }
    if (isRecord(value)) {
      const out: Record<string, unknown> = {};
      for (const [key, val] of Object.entries(value)) {
        const resolvedKey =
          typeof key === "string"
            ? ((yield* resolveSnippetStrings(key, snippets, seen)) as string)
            : key;
        out[resolvedKey] = yield* resolveSnippetStrings(val, snippets, seen);
      }
      return out;
    }
    if (Array.isArray(value)) {
      const out: unknown[] = [];
      for (const item of value) {
        out.push(yield* resolveSnippetStrings(item, snippets, seen));
      }
      return out;
    }
    return value;
  });

const autoDiscover = (
  raw: Record<string, unknown>,
  baseDir: string,
): Effect.Effect<void, LoaderError, never> =>
  Effect.gen(function* () {
    const discovered = yield* discover(baseDir).pipe(
      Effect.mapError((error) => new LoaderError({ reason: error.reason })),
    );
    if (discovered.snippets.length > 0) {
      const snippets = isRecord(raw.snippets) ? { ...raw.snippets } : {};
      for (const entry of discovered.snippets) {
        const text = yield* readTextFile(entry.path);
        const parsed = yield* parseYaml(text);
        snippets[entry.name] = parsed;
      }
      raw.snippets = snippets;
    }
    if (discovered.nodes.length > 0) {
      const nodeList: unknown[] = [];
      for (const entry of discovered.nodes) {
        const text = yield* readTextFile(entry.path);
        const parsed = yield* parseYaml(text);
        if (Array.isArray(parsed)) {
          nodeList.push(...parsed);
        } else if (isRecord(parsed)) {
          nodeList.push(parsed);
        }
      }
      const existing = Array.isArray(raw.nodes) ? raw.nodes : [];
      raw.nodes = [...nodeList, ...existing];
    }
    if (discovered.config.length > 0) {
      let config = isRecord(raw.config) ? { ...raw.config } : {};
      for (const filePath of discovered.config) {
        const text = yield* readTextFile(filePath);
        const parsed = yield* parseYaml(text);
        if (isRecord(parsed)) {
          config = deepMerge(config, parsed) as Record<string, unknown>;
        }
      }
      raw.config = config;
    }
    if (discovered.events.length > 0) {
      const schemas = isRecord(raw.payload_schemas) ? { ...raw.payload_schemas } : {};
      for (const entry of discovered.events) {
        const text = yield* readTextFile(entry.path);
        const parsed = yield* parseJson(text);
        if (isRecord(parsed)) {
          schemas[entry.name] = parsed;
        }
      }
      raw.payload_schemas = schemas;
    }
  });

export const loadRaw = (
  text: string,
  baseDir: string,
): Effect.Effect<Record<string, unknown>, LoaderError, never> =>
  Effect.gen(function* () {
    const parsed = yield* parseYaml(text);
    if (!isRecord(parsed)) {
      return yield* Effect.fail(new LoaderError({ reason: "workflow must be a mapping" }));
    }
    let raw = parsed;
    yield* autoDiscover(raw, baseDir);
    raw = (yield* resolveIncludeDict(raw, baseDir, new Set<string>())) as Record<string, unknown>;
    let snippets: Record<string, unknown> = {};
    if ("snippets" in raw) {
      if (isRecord(raw.snippets)) {
        snippets = yield* resolveSnippetDefinitions(raw.snippets);
      } else {
        return yield* Effect.fail(new LoaderError({ reason: "'snippets' must be a mapping" }));
      }
    }
    raw = (yield* resolveSnippetRefs(raw, snippets, new Set<string>())) as Record<string, unknown>;
    raw = (yield* resolveSnippetStrings(raw, snippets, new Set<string>())) as Record<
      string,
      unknown
    >;
    raw = (yield* resolveIncludeStrings(raw, baseDir, new Set<string>())) as Record<
      string,
      unknown
    >;
    return raw;
  });

export const loadWorkflowFromText = (
  text: string,
  baseDir = process.cwd(),
): Effect.Effect<Workflow, LoaderError, never> =>
  Effect.gen(function* () {
    const raw = yield* loadRaw(text, baseDir);
    return Workflow.fromDict(raw);
  });

export const loadWorkflow = (filePath: string): Effect.Effect<Workflow, LoaderError, never> =>
  Effect.gen(function* () {
    const text = yield* readTextFile(filePath);
    const baseDir = path.dirname(path.resolve(filePath));
    const raw = yield* loadRaw(text, baseDir);
    return Workflow.fromDict(raw);
  });
