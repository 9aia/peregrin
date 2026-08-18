# Architecture (Digest)

- `apps/` run, `packages/` are shared, `tools/` are build helpers, `repos/` is read-only reference.
- `apps` depend on `packages`; `packages` do not depend on `apps`.
- Define services with `Context.Tag` or `Effect.Service`. Concrete layers in `Layer`.
- One public idea per package. Use `exports` in `package.json`.
- Pure logic in `packages`; wiring in `apps`.
