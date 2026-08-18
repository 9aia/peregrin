# Error Management Rules

Errors are split into expected (typed, recoverable) and unexpected (defects, unrecoverable).

## Two types of errors

- **Expected errors (recoverable)** — anticipated failures that are part of the domain: file not found, modpack missing, version conflict. Tracked in the `Effect` error channel `Effect<A, E>`. The caller decides how to handle them.
- **Unexpected errors (defects)** — unrecoverable conditions outside normal flow: invariant violations, corrupted state, logic errors. These are bugs, not domain failures. Represented by `Effect.die` or `Cause.defect`.

## Principles

- Library functions return `Effect<A, E>` for expected errors, never throw.
- No `throw` or `try`/`catch` in library code. Use `Effect.try` / `Effect.tryPromise` to lift exceptions into `Effect`.
- No stringly-typed errors. Use typed errors (`Data` classes or `Schema` tagged structs) so callers can match on variants.
- No silent failures. If something fails, return an `Effect` failure — don't log-and-continue.
- Don't log and return an error. If you return `Effect.fail`, the caller handles it. Log only at the top-level boundary.

## Error types

- Use `Data` or `Schema.Class` for error records. One `Error` type per package, variants per failure mode.
- Use `Effect.fail` with a typed error. Never `Effect.fail("not found")`.
- Error messages describe what failed and what was expected: `ModpackNotFoundError({ modpack: "coindash" })`, not `"not found"`.

## Error propagation

- Use `yield*` for propagation. Don't catch-and-rethrow with a generic message.
- `yield*` short-circuits on the first error. Don't manually check each step.
- Add context at layer boundaries: `resolveInstance` errors become `LauncherError` with a cause, not a bare passthrough.
- Don't wrap errors unnecessarily. If a function's error type is the same as the callee's, `yield*` without wrapping.
- Use `Effect.mapError` to transform errors when crossing package boundaries. Don't leak internal error types.

## Converting failures to defects

- If recovery is impossible at a certain layer, convert `Effect` failure to a defect with `Effect.orDie` and a descriptive message.
- Only do this in `main.ts` or when an invariant guarantees success. Library code should always propagate.
- This converts a recoverable error into an unrecoverable defect — the error type is removed and the program terminates.

## At boundaries

- `main.ts` / top-level command handlers: convert `Effect` failure to user-facing output. This is the only place `Console.error` should be emitted.
- Tests: `Effect.runPromiseExit` or `Effect.runSync` are fine — test failures should be loud.
