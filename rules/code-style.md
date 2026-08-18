# Code Style Rules

Style rules for writing clear, maintainable TypeScript + Effect code.

## Explicit over clever

- Write explicit code, not point-free. `pipe(x, transform)` is fine; avoid deeply nested calls.
- No "clever" one-liners that sacrifice readability. A 3-line version that reads top-to-bottom is better than a 1-line version that requires parsing.
- Prefer `Effect.gen` for sequential effectful code. Use `pipe` for linear data transformations:
  ```ts
  const result = pipe(
    items,
    Array.filter((x) => x.isActive),
    Array.map((x) => x.name),
    Array.join(", "),
  );
  ```

## Naming

- Functions are verbs: `resolveInstance`, `loadModpack`, `createInstance`. Not `instance`, `modpackData`.
- Booleans are predicates: `isActive`, `hasMods`, `canLaunch`. Not `active`, `mods`, `launch`.
- Types are nouns: `Instance`, `ModpackConfig`, `LauncherPaths`.
- Enums are nouns, variants are descriptive: `enum LoadError { NotFound, PermissionDenied, Corrupted }`.
- No abbreviations: `config` not `cfg`, `instance` not `inst`, `modpack` not `mp`.

## Pattern matching

- Use `Match.value(x).pipe(...)` or `Match.tag(x)` for multi-branch logic on unions. Effect's `Match` is exhaustive.
- Use `Option.match` / `Either.match` for `Option`/`Either` extraction.
- Use `if` guards for early returns: `if (Option.isNone(opt)) return Effect.fail(...);`.

## Branded types

- Wrap domain strings/numbers in a branded type to prevent mixups: `type InstanceName = string & Brand.Brand<"InstanceName">`.
- The compiler prevents passing an `InstanceName` where a `ModpackName` is expected.
- Use `Schema.Brand` to build, validate, and combine branded types.
- Don't over-wrap. A `Count` branded `number` adds noise. Use branded types when two values share a primitive but different semantics.

## Reducing nesting

- Early returns over deep nesting. Guard clauses at the top of a function.
- `yield*` over nested `then`/`catch` for effectful code.
- Max nesting depth: 3 levels. If you're 4+ levels deep, extract a helper function.

## Function length

- Functions should fit on one screen (~30 lines). If longer, look for extraction points.
- Don't extract trivially small functions (1-2 lines) unless the name adds clarity.
- A function does one thing. If the function name needs "and", it does two things: `loadAndValidate` → `load` + `validate`.

## What to avoid

- **Tacit/point-free style** — `xs.map(transform)` instead of `xs.map((x) => transform(x))` in non-pipe contexts.
- **Deep nesting** — 4+ levels of `if`/`match`. Extract or early-return.
- **Catch-all `_` in match** — use explicit arms; add a catch-all only for genuinely open unions.
- **Abbreviated names** — `cfg`, `inst`, `mp`. Spell it out.
- **Commenting out code** — delete it. Git remembers.
