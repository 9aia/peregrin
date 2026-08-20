import { Data, Effect } from "effect";
import fs from "node:fs";
import path from "node:path";

export interface Entry {
  readonly name: string;
  readonly path: string;
}

export class DiscoverError extends Data.TaggedError("DiscoverError")<{
  readonly reason: string;
}> {}

export class DiscoveredFiles {
  constructor(
    readonly nodes: ReadonlyArray<Entry> = [],
    readonly snippets: ReadonlyArray<Entry> = [],
    readonly events: ReadonlyArray<Entry> = [],
    readonly handlers: ReadonlyArray<Entry> = [],
    readonly config: ReadonlyArray<string> = [],
    readonly secrets: ReadonlyArray<Entry> = [],
    readonly env: ReadonlyArray<string> = [],
    readonly functions: ReadonlyArray<Entry> = [],
  ) {}
}

const SUFFIX_KINDS: ReadonlyArray<readonly [string, string]> = [
  [".node.yml", "node"],
  [".node.yaml", "node"],
  [".snippet.yml", "snippet"],
  [".snippet.yaml", "snippet"],
  [".event.json", "event"],
  [".handler.py", "handler"],
  [".config.yml", "config"],
  [".config.yaml", "config"],
  [".secret.yml", "secret"],
  [".secret.yaml", "secret"],
  [".fn.yml", "function"],
  [".fn.yaml", "function"],
];

const matchSuffix = (filename: string): readonly [string, string] | null => {
  for (const [suffix, kind] of SUFFIX_KINDS) {
    if (filename.endsWith(suffix)) {
      const name = filename.slice(0, -suffix.length);
      if (name.length > 0) {
        return [name, kind];
      }
    }
  }
  return null;
};

const isDirectory = (target: string): Effect.Effect<boolean, never, never> =>
  Effect.try(() => fs.statSync(target).isDirectory()).pipe(
    Effect.catchAll(() => Effect.succeed(false)),
  );

const readDir = (dir: string): Effect.Effect<ReadonlyArray<fs.Dirent>, DiscoverError, never> =>
  Effect.try(() => fs.readdirSync(dir, { withFileTypes: true })).pipe(
    Effect.mapError((error) => new DiscoverError({ reason: String(error) })),
  );

const collect = (
  dir: string,
  skipAc: boolean,
): Effect.Effect<ReadonlyArray<readonly [string, string]>, DiscoverError, never> =>
  Effect.gen(function* () {
    const entries = yield* readDir(dir);
    const sorted = [...entries].sort((a, b) => a.name.localeCompare(b.name));
    const files: Array<readonly [string, string]> = [];
    const dirs: Array<string> = [];
    for (const entry of sorted) {
      if (entry.isDirectory()) {
        if (entry.name === ".ac" && skipAc) continue;
        if (entry.name.startsWith(".")) continue;
        dirs.push(entry.name);
      } else if (entry.isFile()) {
        files.push([path.join(dir, entry.name), entry.name] as const);
      }
    }
    const out: Array<readonly [string, string]> = [...files];
    for (const d of dirs) {
      const sub = yield* collect(path.join(dir, d), false);
      out.push(...sub);
    }
    return out;
  });

export const discover = (baseDir: string): Effect.Effect<DiscoveredFiles, DiscoverError, never> =>
  Effect.gen(function* () {
    const acDir = path.join(baseDir, ".ac");
    const acExists = yield* isDirectory(acDir);
    const roots: Array<string> = [];
    if (acExists) roots.push(acDir);
    roots.push(baseDir);
    const seen: Record<string, Map<string, string>> = {
      node: new Map(),
      snippet: new Map(),
      event: new Map(),
      handler: new Map(),
      secret: new Map(),
      function: new Map(),
    };
    const nodes: Array<Entry> = [];
    const snippets: Array<Entry> = [];
    const events: Array<Entry> = [];
    const handlers: Array<Entry> = [];
    const config: Array<string> = [];
    const secrets: Array<Entry> = [];
    const env: Array<string> = [];
    const functions: Array<Entry> = [];
    for (const root of roots) {
      const collected = yield* collect(root, root !== acDir);
      for (const [fpath, fname] of collected) {
        if (fname === ".env") {
          env.push(fpath);
          continue;
        }
        const matched = matchSuffix(fname);
        if (matched === null) continue;
        const [name, kind] = matched;
        if (kind in seen) {
          const map = seen[kind];
          if (map.has(name)) continue;
          map.set(name, fpath);
          const entry: Entry = { name, path: fpath };
          if (kind === "node") nodes.push(entry);
          else if (kind === "snippet") snippets.push(entry);
          else if (kind === "event") events.push(entry);
          else if (kind === "handler") handlers.push(entry);
          else if (kind === "secret") secrets.push(entry);
          else if (kind === "function") functions.push(entry);
        } else if (kind === "config") {
          config.push(fpath);
        }
      }
    }
    config.sort();
    return new DiscoveredFiles(nodes, snippets, events, handlers, config, secrets, env, functions);
  });
