# Workspace Rules

Rules for the Vite+ Bun workspace root.

## Tooling

- Use `vp` for all tasks: `vp install`, `vp check`, `vp test`, `vp build`, `vp run <task>`.
- Use `bun` as the package manager. `package.json` `devEngines.packageManager` is `bun`.
- Run `vp install` after pulling remote changes and before starting work.

## Workspace layout

- `package.json` declares `workspaces: ["packages/*", "apps/*", "tools/*"]`.
- Shared dependencies use workspace `catalog:` where appropriate.
- Every workspace package has `package.json`, `tsconfig.json`, and a `README.md`.

## Adding a package

1. Create `packages/<name>/` or `apps/<name>/`.
2. Add `package.json` with `name`, `version`, `type: "module"`, and `exports` for packages.
3. Add `tsconfig.json` extending the root `tsconfig.json` if needed.
4. Add a `README.md` following `rules/templates/package-readme.md`.
5. Run `vp install` from the workspace root.

## Project worktree

- `project/` is a git worktree on the orphan `project` branch.
- `project/` is git-ignored in `master` and other non-project branches.
- Project planning and decision material lives there:
  - `TODO.md` — active tasks.
  - `BACKLOG.md` — deferred ideas.
  - `notes/` — working notes.
  - `plans/` — plans and milestones.
  - `adrs/` — Architecture Decision Records.

## Validation

- Run `vp check` to format, lint, and type check.
- Run `vp test` and `vp build` before opening a PR.
- Use `vp run ready` to run the full `ready` pipeline.
