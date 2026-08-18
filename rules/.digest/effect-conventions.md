# Effect Conventions (Digest)

- Use `Effect.gen` + `yield*` for sequential effectful code.
- Use `pipe` for linear data and effect transformations.
- Define services with `Context.Tag` / `Effect.Service` and wire with `Layer`.
- Use `Effect.acquireRelease` and `Scope` for resource safety.
- Return typed errors with `Effect.fail`. Use `Effect.mapError` at boundaries.
- No `async`/`await` mixed with `Effect`. No `Effect.runSync` in libraries.
- Use `Effect.log` inside effects; no bare `console.log` in library code.
