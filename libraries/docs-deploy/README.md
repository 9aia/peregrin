# docs-deploy

Docs-site deployment as a peregrin bundle (PL-009 / P-009): on a
verified `push` to a watched repo's `refs/heads/docs` → detached
worktree at the pushed sha → `bun install` + the repo's own
`docs:deploy` (vitepress build → `gh-pages`) → GitHub commit status
under `docs/deploy`. This is the campaign-shaped replacement for the
GHA `deploy.yml` workflows deleted org-wide in C3.

## v1 scope (D1 audit)

Only repos whose `docs:deploy` is **self-contained** in the docs
tree — the `docs` orphan branch *is* the site checkout
(`package.json` + `bun.lock` + `.vitepress/` at its root), so the
worktree root is the build root:

| repo | `docs:deploy` | verdict |
| --- | --- | --- |
| `9aia/sepia` | `bun run build && bunx gh-pages -d .vitepress/dist` | v1 |
| `9aia/peregrin` | same | v1 |
| `9aia/lasius` | `bun run docs:build && bunx gh-pages -d .vitepress/dist` | v1 |

**Deferred — gen-dependent** (deploy runs `docs:gen`/`docs:api`
which `cd ..` into the main checkout for `cargo run`/`cargo doc` —
needs dual-checkout support, PL-009 D6): `myco`, `tamias`, `hypha`,
`sturn`, `kuda`, `mimus`, `bupha`, `cuora`, `pavo`.

**Deferred — `9aia.github.io`**: the site lives on `main` (no `docs`
branch), the toolchain is pnpm (`pnpm-lock.yaml`, no `docs:deploy`
script), and the deleted `deploy.yml` used `actions/deploy-pages`
artifact deploy — Pages-source semantics don't map onto `gh-pages`
without an operator decision.

## Mechanics

- **Ingress**: `docs.ingress.webhook` (`github-webhook`) watches the
  three repos; `secret_env` names follow the existing
  `BUPHA_WH_<OWNER>_<REPO>` convention — *names only*, granted via
  `capabilities.secrets` (`BUPHA_TOKEN` + the three `BUPHA_WH_*`).
  `~/.config/peregrin/env` must carry values for
  `BUPHA_WH_9AIA_SEPIA`/`BUPHA_WH_9AIA_LASIUS` (only
  `BUPHA_WH_9AIA_PEREGRIN` exists today) plus `BUPHA_TOKEN`.
- **Routing**: `docs.route` re-emits every verified push as
  `push.<repo>.<ref>.<clone_url>.<sha>`; per-repo pipelines subscribe
  `push.<repo>.refs/heads/docs.**.*` — main-branch pushes route but
  match nothing (ci-gate's territory).
- **Deploy**: `driver: git` materializes a detached worktree at the
  pushed sha (clone cache under `worktree_base/.repos`, creds via
  argv-scoped `http.extraHeader` — never persisted); the inner shell
  runs `config.cmd` at the worktree root.
- **gh-pages auth**: the worktree's `origin` is the reported
  `clone_url`, so `gh-pages` detects the repo itself. The shipped cmd
  exports an env-scoped `credential.helper` (reads `$BUPHA_TOKEN`)
  plus author/committer env — the token never lands in any git
  config, and `GIT_CONFIG_*` env propagates into gh-pages' internal
  git invocations. If you override `cmd`, keep that export.
- **Statuses**: `pending` on gate, verdict (`done`→success,
  `failed`→failure, `failed.<sha>` materialization→error via
  `docs.failmap`) under context `docs/deploy`.
- **Supersede**: `{repo}:{ref}` latest-wins, `cancel_running: true`.

## Install + deploy (tamias provider flow)

```sh
tamias install ./libraries/docs-deploy
tamias deploy docs-deploy        # → docs-deploy-campaign.service
```

`tamias install`/`deploy` dispatch `tamias-driver-peregrin`, which
writes `~/.config/peregrin/docs-deploy.yaml` (`include:` of the
bundle's `main.yaml`). Because the campaign yaml ships its own
webhook node, `provider.ingress` deliberately declares **no**
`hooks[]` — otherwise deploy would generate a second ingress node
(`PEREGRIN_WH_*` names) and every push would double-deliver.

**Production wiring (PL-009 D5, not done here):** the intended live
topology is adding this bundle's `main.yaml` to the existing
`campaign serve` invocation next to `ci-gate-full.yaml` — the shared
`SharedWebhook` demuxes verified pushes by `repository.full_name`, so
one listener serves both campaigns (per-node `listen`/`path` are
ignored under `serve`). The `docs-deploy-campaign.service` unit is
the standalone fallback on `127.0.0.1:29174`.

Forge-hook registration (pointing each repo's webhook at the public
URL) and adding the `BUPHA_WH_9AIA_SEPIA`/`BUPHA_WH_9AIA_LASIUS`
values to `~/.config/peregrin/env` are operator steps.

## Params (validated against `params.schema.json`)

| Key | Default | Meaning |
| --- | --- | --- |
| `worktree_base` | `~/.local/share/peregrin/worktrees` | `driver-git` `base` |
| `token_env` | `BUPHA_TOKEN` | Secret **name** for clone auth + gh-pages push + statuses |
| `timeout` | `600` | Inner shell timeout, seconds |
| `context` | `docs/deploy` | Commit-status context |
| `cmd` | see `main.yaml` | Deploy command at the worktree root |
