---
name: ci-cd
description: CI/CD pipeline rules for the Peregrin project. Use when configuring CI, writing pipeline files, or debugging CI failures.
---

# CI/CD

## Required checks

- Format: `vp fmt`.
- Lint: `vp lint`.
- Type check: `vp check`.
- Tests: `vp test` for all workspace packages.
- Build: `vp build` for all `apps/` and `packages/`.

## Pipeline order

1. `vp install` to restore dependencies.
2. `vp check` for format, lint, and type checks.
3. `vp run -r test` to run tests across the workspace.
4. `vp run -r build` to build all packages.
5. `vp run ready` for the full `ready` pipeline.

## Secrets

- Never commit secrets, tokens, or API keys.
- Use `.env` files and keep them out of git.
