# Dependency Management (Digest)

- Use `vp install` from root. Don't hand-edit `bun.lock`.
- `bun add <pkg>` in the target package. `bun add -d` for dev deps.
- Prefer workspace `catalog:` for shared `vite`, `vite-plus`, `typescript`.
- Core allowed: `effect`, `@effect/*`, `vite`, `vite-plus`, `vitest`, `@effect/vitest`.
- Avoid `latest` or unbounded ranges. Prefer packages published 7+ days.
