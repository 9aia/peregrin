# ci-gate

The bupha CI gate as a peregrin bundle (ADR-0080/ADR-0081): on a
`push` event → detached worktree at the pushed sha → inner `shell`
run → GitHub commit status under `ci/<campaign>`.

## Install + enable

```sh
peregrin install git:9aia/peregrin/libraries/ci-gate
# or, once P6 lands:
peregrin create --template ci-gate
```

The bundle is the *policy* graph only. The node config (a workflow
document itself, per ADR-0080) is where the operator binds ingress and
secrets — `peregrin create --template ci-gate` materializes a thin
config like the one below.

## Operator node config

```yaml
name: my-gate

include:
  - <store>/ci-gate/main.yaml   # or a `bundle:ci-gate` ref once wired

config:
  name: ci-gate                     # statuses post as ci/<name>
  repo: "9aia/peregrin"             # repo to gate ("" = every watched repo)
  ref: "refs/heads/main"
  cmd: "bash scripts/ci.sh"
  worktree_base: "~/ci/worktrees"
  token_env: "GITHUB_TOKEN"         # secret NAME, never the value
  timeout: 600

nodes:
  # The ingress half — node-level, never shipped in the bundle.
  - id: gate.ingress
    driver: github-webhook
    on: [start]
    params:
      listen: "127.0.0.1:29173"
      repos:
        - name: "9aia/peregrin"
          secret_env: "BUPHA_WH_9AIA_PEREGRIN"   # NAME of the webhook secret
```

A `driver: sturn-poll` node works as the source instead — it emits
the same `push` shape (`snap` → run, no webhook round-trip).

## Params (validated against `params.schema.json`)

| Key | Default | Meaning |
| --- | --- | --- |
| `name` | `ci-gate` | Status context base → `ci/<name>` |
| `repo` | `""` | `owner/name` to gate; empty gates all watched repos |
| `ref` | `refs/heads/main` | Ref to gate |
| `status_repo` | `""` | Post statuses to a different repo (bupha redirect) |
| `cmd` | `bash scripts/ci.sh` | Gate command, run inside the worktree |
| `worktree_base` | `~/.local/share/peregrin/worktrees` | `driver-git` `base` |
| `token_env` | `GITHUB_TOKEN` | Secret **name** for the forge token (clone + status) |
| `timeout` | `600` | Inner shell timeout, seconds |

Ingress params live on the webhook node (`repos[].name`,
`repos[].secret_env`, `listen`, `path`, `api_base`) — see
`docs/reference/` and `peregrin-driver-github-webhook`'s doc-comment.

## Event routing

`on:` matches event *types* only, so `ci-gate.match` re-emits a gated
push as `push.gated.<repo>.<clone_url>.<sha>` — fields ride the type and
downstream nodes recover them via `${event.match.1}`/`${event.match.2}`
(`**` re-joins clone_url's `.`-separated segments).

`node.failed` events carrying `{node, sha}` (driver-git
materialization failures) are re-mapped onto `<node>.failed.<sha>` by
`ci-gate.failmap` so a broken checkout posts a status (`state: error`)
instead of going silent.

Multi-repo gates: this campaign gates one `repo`+`ref` — for a fleet
of repos see `peregrin import bupha`, which emits per-repo routed
subscriptions (`push.<repo>.<ref>.<clone_url>.<sha>`) directly.
