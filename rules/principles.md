# Principles

General coding principles for all code in the workspace.

## Type safety

- Let the compiler and `Schema` enforce invariants. Use discriminated unions over `string` when the set of values is known. Use `Option`/`Either` over `null`/`undefined`.
- Branded types for domain strings and numbers: `type UserId = string & Brand.Brand<"UserId">` prevents passing a modpack id where a user id is expected.
- No `throw` in library code. `throw` is acceptable in tests and `main.ts` only.
- No `any`. Prefer `unknown` for unknown data and `Schema` to decode.
- Use `Schema` at boundaries. After validation, code can assume well-formed data.

## Robustness

- Handle errors at boundaries. Library functions return `Effect<A, E>`; binaries decide how to present errors to the user.
- No silent failures. If something goes wrong, return a typed `Effect` error or log — don't swallow and continue.
- Degrade gracefully. If an optional feature is unavailable, fall back with a warning and continue — don't crash.
- Validate input at the edges. Trust data once it has crossed a boundary (file load, user input, network).

## Resilience

- No unhandled exceptions in library code. A library should never crash the host process. Return `Effect` for recoverable errors.
- Resource cleanup via `Scope`, not manual close calls. If a resource needs cleanup, use `Effect.acquireRelease` or `Layer` scoping.
- Bounded retries for transient failures. Don't retry infinitely. Don't retry non-transient failures.

## Elegance

- Fewer moving parts. If a function can be expressed in 5 lines without sacrificing clarity, don't split it into 3 helper functions.
- Names reveal intent. `resolveInstance` not `processConfig`. `UserStore` not `UserManager`.
- Symmetry over cleverness. If `create` exists, `delete` exists. If `save` returns `Effect`, so does `load`.
- Prefer composition over inheritance. Services for behavior, data types for data, `Layer` for wiring.

## Extensibility

- Built-ins emerge from the community. New capabilities start as declarative, YML-driven extension points (driver presets, functions, adapters) and are promoted to built-in, compiled code only when they are widely used or performance-critical.
- Declarative first. Prefer solving a problem in a workflow, driver preset, or function file before adding a new package or runtime feature.
- Graceful promotion. A YML extension that becomes a bottleneck must be easy to rewrite as a TypeScript binding without changing the user-facing surface. Names, parameters, and semantics must stay stable across the boundary.
