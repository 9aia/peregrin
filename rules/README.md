# Rules

Project-wide conventions for structure, naming, and documentation in the Peregrin TypeScript + Effect + Bun workspace.

## Files

| File                       | Scope                                                                            |
| -------------------------- | -------------------------------------------------------------------------------- |
| `principles.md`            | General coding principles — type safety, robustness, resilience, elegance        |
| `architecture.md`          | Structural rules — dependency direction, ports & adapters, modularity, layering  |
| `code-style.md`            | Rules for naming, pattern matching, branded types, nesting, function length      |
| `data-types.md`            | Rules for choosing types — branded types, collections, time, `Option`/`Either`   |
| `error-management.md`      | Rules for `Effect` error channel, typed errors, boundary handling                |
| `testing.md`               | Rules for tests across packages and apps                                         |
| `effect-conventions.md`    | Rules for writing idiomatic Effect code — `Effect.gen`, `pipe`, layers, services |
| `dependency-management.md` | Rules for adding deps, versioning policy, allowed packages                       |
| `workspace.md`             | Rules for the workspace root — Vite+ monorepo, `apps/` and `packages/`           |
| `git-conventions.md`       | Rules for branch naming, commit messages, pull requests                          |
| `ci-cd.md`                 | Rules for CI pipeline stages, required checks                                    |
| `comments.md`              | Rules for comments and JSDoc                                                     |

## Relationship

```
principles.md                     — applies to all code (modules → workspace)
architecture.md                   — applies to all structure (modules → workspace)
code-style.md                     — applies to all code (modules → workspace)
data-types.md                     — applies to all code (modules → workspace)
error-management.md               — applies to all code (modules → workspace)
effect-conventions.md             — applies to all code (modules → workspace)
testing.md                        — applies to all packages and apps
workspace.md                      — applies to the workspace root
dependency-management.md          — applies to workspace root and package.json files
git-conventions.md                — applies to all commits/branches/PRs
ci-cd.md                          — applies to CI pipeline and automated checks
comments.md                       — applies to all code (modules → workspace)
```
