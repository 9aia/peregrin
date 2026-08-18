# Error Management (Digest)

- Expected errors → `Effect<A, E>`. Caller decides handling.
- Unexpected errors → defects via `Effect.die`/`Cause.defect`.
- No `throw` in library code. No `try`/`catch` to recover — use `Effect.try` to lift.
- No stringly-typed errors. Use `Data` or `Schema.Class` error records.
- No silent failures. Don't log-and-continue.
- Use `yield*` for propagation. Use `Effect.mapError` at boundaries.
- Convert `Effect` failure to a defect only in `main.ts` or with an invariant.
