# Git Conventions

Rules for branches, commits, and pull requests.

## Branches

- `main` is the default branch.
- Feature branches: `feat/<short-name>` or `feature/<short-name>`.
- Fix branches: `fix/<short-name>`.
- Experiment branches: `experiment/<short-name>`.

## Commits

- Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`.
- Subject should finish the sentence "If applied, this commit will ...".
- Keep the body focused on why, not what. The diff shows what.
- Run `vp check` before committing.

## Pull requests

- Use `gh pr create` for GitHub PRs.
- Fill the template with summary, test plan, and any breaking changes.
- Use `vp run ready` before marking a PR ready for review.
