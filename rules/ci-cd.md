# CI/CD Rules

Rules for the continuous integration and delivery pipeline.

## Required checks

- Format: `vp fmt` (Oxfmt).
- Lint: `vp lint` (Oxlint with vite-plus rules).
- Type check: `tsc` or `vp check`.
- Tests: `vp test` for all workspace packages.
- Build: `vp build` for all `apps/` and `packages/`.

## Pipeline stages

1. `vp install` to restore dependencies.
2. `vp check` for format, lint, and type checks.
3. `vp run -r test` to run tests across the workspace.
4. `vp run -r build` to build all packages.
5. `vp run ready` to run the full `ready` pipeline.

## Secrets

- Never commit secrets, tokens, or API keys.
- Use environment-specific `.env` files and keep them out of git.
