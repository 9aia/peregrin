---
name: workspace-structure
description: Workspace structure rules for the Peregrin project. Use when modifying workspace-level configuration, project layout, or `package.json` files.
---

# Workspace Structure

## Layout

- `apps/` — runnable applications.
- `packages/` — shared libraries.
- `tools/` — build helpers and scripts.
- `repos/` — vendored reference repositories (read-only).

## Configuration

- Root `package.json` declares workspaces and catalog overrides.
- Each package has its own `package.json`, `tsconfig.json`, and `README.md`.
- Shared deps use `catalog:` where appropriate.

## Dependency direction

- `apps` depend on `packages`. `packages` do not depend on `apps`.
- No circular dependencies.
