# docs-deploy

Docs-site deployment as a peregrin bundle (PL-009 / P-009): on a
verified `push` to a watched repo's `refs/heads/docs` (or
`refs/heads/main` for `9aia.github.io`, whose site *is* main) →
detached worktree at the pushed sha → `bun install` + the repo's own
`docs:deploy` (vitepress build → `gh-pages`) → GitHub commit status
under `docs/deploy`. This is the campaign-shaped replacement for the
GHA `deploy.yml` workflows deleted org-wide in C3.

## v1 scope (D1 audit + D6 dual-checkout)

Two repo shapes, one campaign:

- **Self-contained** — the `docs` orphan branch *is* the site
  checkout (`package.json` + `bun.lock` + `.vitepress/` at its
  root), so the worktree root is the build root: `9aia/sepia`,
  `9aia/peregrin`, `9aia/lasius`.
- **Gen-dependent** — `docs:deploy` runs `docs:gen`/`docs:api`
  steps that `cd ..` into the main checkout for `cargo run` /
  `cargo doc`: `9aia/myco`, `9aia/tamias`, `9aia/cuora`, `9aia/pavo`,
  `9aia/hypha`, `9aia/sturn`, `9aia/kuda`, `9aia/mimus`, `9aia/bupha`.
  Their git nodes set `params.dest: "docs-<repo>-<sha>/docs"` +
  `params.extra_worktrees: [{ref: "${config.main_ref}"}]` (driver-git
  D6): the docs-branch worktree lands at `<run>/docs`, a second
  worktree of `refs/heads/main` at `<run>`, and the inner shell's
  cwd stays the docs worktree — exactly the dev layout (`repo/` +
  `repo/docs/` worktree), so `cd ..` hits the main checkout and the
  gen scripts' `docs/…` output paths land back in the docs tree.
  Both checkouts are destroyed with the run (`keep` retains both).
- **Main-branch site** — `9aia/9aia.github.io`: the org site lives on
  `main` (no `docs` branch), so its triplet subscribes
  `push.9aia/9aia.github.io.refs/heads/main.**.*`. Self-contained —
  the repo root IS the site (`bun install` + `docs:deploy` =
  `vitepress build` → `bunx gh-pages -d dist`). Its `secret_env` is
  `BUPHA_WH_9AIA_GITHUB_IO` (dots in the repo name can't ride an env
  var). **Operator step:** the repo's Pages source is currently
  `build_type: workflow` (the deleted `deploy.yml`'s
  `actions/deploy-pages`); switch it once to *Deploy from a branch →
  `gh-pages` / (root)* — the built output is identical, only the
  delivery mechanism changes.

The shipped `cmd` exports a persistent per-host
`CARGO_TARGET_DIR=$HOME/.local/share/peregrin/cargo-target` — the
worktrees are ephemeral, so without it every deploy cold-builds the
gen step and can exceed the 600s inner-shell cap. (mimus's
`docs:api` pins `CARGO_TARGET_DIR=target` itself, so it stays
cold — a repo-side fix, not ours.)

**Converted — `9aia.github.io`** (was deferred): migrated pnpm → bun
(`bun.lock`, `docs:deploy` script on the repo's `ghpages-conv`
branch), and its triplet watches `refs/heads/main`. Remaining ops:
create the `BUPHA_WH_9AIA_GITHUB_IO` webhook secret + forge hook, and
switch the repo's Pages source from Actions-artifact to the
`gh-pages` branch (see v1 scope above).

## Mechanics

- **Ingress**: `docs.ingress.webhook` (`github-webhook`) watches the
  thirteen repos; `secret_env` names follow the existing
  `BUPHA_WH_<OWNER>_<REPO>` convention (dots sanitized —
  `9aia.github.io` → `BUPHA_WH_9AIA_GITHUB_IO`) — *names only*, granted via
  `capabilities.secrets` (`BUPHA_TOKEN` + the thirteen `BUPHA_WH_*`).
  `~/.config/peregrin/env` must carry a `BUPHA_WH_9AIA_<REPO>` value
  per watched repo plus `BUPHA_TOKEN`. As of D6 the env file already
  has `BUPHA_WH_9AIA_{BUPHA,CUORA,KUDA,MIMUS,MYCO,PEREGRIN,STURN}`;
  ops must add `BUPHA_WH_9AIA_{SEPIA,LASIUS,TAMIAS,PAVO,HYPHA}` plus
  `BUPHA_WH_9AIA_GITHUB_IO` (and
  confirm `BUPHA_TOKEN`). The webhook node resolves every listed
  `secret_env` at startup, so all thirteen values must exist before
  deploying — a repo whose secret is missing fails the whole
  ingress, not just its own pushes.
- **Routing**: `docs.route` re-emits every verified push as
  `push.<repo>.<ref>.<clone_url>.<sha>`; per-repo pipelines subscribe
  `push.<repo>.refs/heads/docs.**.*` — except `9aia.github.io`, which
  subscribes `refs/heads/main` — so main-branch pushes on the other
  repos route but match nothing (ci-gate's territory).
- **Deploy**: `driver: git` materializes a detached worktree at the
  pushed sha (clone cache under `worktree_base/.repos`, creds via
  argv-scoped `http.extraHeader` — never persisted); the inner shell
  runs `config.cmd` at the worktree root. Gen-dependent repos add a
  second `extra_worktrees` checkout of `main_ref` at the run root —
  see "v1 scope" above.
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
URL) and adding the missing `BUPHA_WH_9AIA_*` values to
`~/.config/peregrin/env` (see Ingress above) are operator steps.

## Params (validated against `params.schema.json`)

| Key | Default | Meaning |
| --- | --- | --- |
| `worktree_base` | `~/.local/share/peregrin/worktrees` | `driver-git` `base` |
| `token_env` | `BUPHA_TOKEN` | Secret **name** for clone auth + gh-pages push + statuses |
| `timeout` | `600` | Inner shell timeout, seconds |
| `context` | `docs/deploy` | Commit-status context |
| `main_ref` | `refs/heads/main` | Ref checked out at `<run>` for gen-dependent repos' dual-checkout |
| `cmd` | see `main.yaml` | Deploy command at the worktree root |
