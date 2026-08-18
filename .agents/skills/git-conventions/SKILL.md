---
name: git-conventions
description: Git branch naming, commit messages, and pull request conventions for the Peregrin project. Use when committing code, creating branches, opening PRs, or writing commit messages.
---

# Git Conventions

## Branches

- `main` is the default branch.
- Feature branches: `feat/<short-name>`.
- Fix branches: `fix/<short-name>`.
- Experiment branches: `experiment/<short-name>`.

## Commits

- Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.
- Subject should finish the sentence "If applied, this commit will ...".
- Body focuses on why, not what.
- Run `vp check` before committing.

## Pull requests

- Use `gh pr create` for GitHub PRs.
- Include summary, test plan, and breaking changes.
- Use `vp run ready` before marking a PR ready for review.
