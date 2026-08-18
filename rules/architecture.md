# Architecture Rules

Rules for package and module structure in the Vite+ monorepo.

## Package structure

- `apps/` — runnable applications (websites, CLIs, services).
- `packages/` — shared libraries consumed by `apps/` or other `packages/`.
- `tools/` — build helpers and internal scripts.
- `repos/` — vendored reference repositories (read-only, not imported).

## Dependency direction

- `apps` depend on `packages`. `packages` never depend on `apps`.
- `packages` depend on other `packages` lower in the dependency graph.
- Circular dependencies are forbidden.
- Prefer `exports` in `package.json` for a public surface. Don't expose internals.

## Ports and adapters

- Define a `Service` (Effect tag) for every capability that crosses a boundary (fs, network, time, crypto).
- The service interface is the port. Concrete implementations are adapters (live, test, mock).
- Services are constructed via `Layer`. Apps wire layers at the composition root.
- Core logic is pure `Effect` and depends on services, not concrete implementations.

## Modularity

- One public idea per package. If a package needs "and" in its name, split it.
- Co-locate data types and operations. `InstanceConfig` lives next to `loadInstanceConfig`.
- Keep platform details in adapter packages. Core packages are platform-agnostic.

## Layering

- `Effect` programs in `apps/`. Pure logic in `packages/`. Adapters in their own package or at the package boundary.
- A package's `src/index.ts` exports the public API. Internal modules are not exported.
- Use `internal` folders for package-private code and exclude from `exports`.
