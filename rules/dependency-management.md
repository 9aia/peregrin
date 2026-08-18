# Dependency Management Rules

Rules for adding and updating dependencies.

## Package manager

- Use `bun` and `vp` for all installs. Do not hand-edit `bun.lock`.
- Run `vp install` from the workspace root. It will install all workspace packages.

## Adding dependencies

- Use `bun add <pkg>` in the target package directory for runtime dependencies.
- Use `bun add -d <pkg>` for dev dependencies.
- For shared deps, add to the root `package.json` and reference with `catalog:` in workspace packages.
- Prefer workspace catalog for `vite`, `vite-plus`, and `typescript`.

## Allowed packages

- Core: `effect`, `@effect/*` for runtime, schemas, testing, vitest integration.
- Bundling: `vite`, `vite-plus`.
- Testing: `vitest`, `@effect/vitest`.
- Utilities: `typescript`, `@types/*` as needed.
- Prefer packages published for at least 7 days. Avoid `latest` or unbounded ranges.

## Audit

- Run `bun install` with `--audit` or `vp outdated` before major changes.
- Do not add dependencies for trivial one-liners. Check `effect/Array`, `effect/Record`, etc. first.
