---
name: create-package
description: How to create a new package under `apps/` or `packages/` in the Peregrin workspace. Use when adding a new app, library, or tool.
---

# Create a New Package

## When to use

- Adding a new app under `apps/`.
- Adding a shared library under `packages/`.
- Adding a build helper under `tools/`.

## Structure

```
apps/<name>/
  package.json
  tsconfig.json
  vite.config.ts (apps)
  src/
    index.ts | main.ts
  README.md
```

## Steps

1. Create the package directory under `apps/`, `packages/`, or `tools/`.
2. Add `package.json` with `name`, `version`, `type: "module"`, and `exports` for libraries.
3. Add `tsconfig.json` extending the root config.
4. Add `README.md` following `rules/templates/package-readme.md`.
5. Run `vp install` from the workspace root.
